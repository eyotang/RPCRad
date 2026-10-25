package main

import (
	"context"
	"crypto/md5"
	"flag"
	"fmt"
	"log"
	"net"
	"strconv"
	"time"

	"github.com/jhump/protoreflect/desc"
	"github.com/jhump/protoreflect/desc/protoparse"
	"github.com/jhump/protoreflect/dynamic"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/health"
	healthpb "google.golang.org/grpc/health/grpc_health_v1"
	"google.golang.org/grpc/metadata"
	"google.golang.org/grpc/reflection"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/reflect/protoregistry"
)

func reply(method *desc.MethodDescriptor, req *dynamic.Message, sequence int) (*dynamic.Message, error) {
	result := dynamic.NewMessage(method.GetOutputType())
	user := map[string]interface{}{
		"id": "usr_1024", "name": "Alex Chen", "email": "alex@example.com",
		"status": "ACTIVE", "created_at": "2026-10-25T08:30:00Z",
	}
	if field := req.FindFieldDescriptorByName("user_id"); field != nil {
		if id, _ := req.GetField(field).(string); id != "" {
			user["id"] = id
		}
	}
	if method.GetName() == "GetUser" && user["id"] == "missing" {
		return nil, status.Error(codes.NotFound, "User not found")
	}
	if method.GetName() == "CreateUser" {
		user["name"] = req.GetFieldByName("name")
		user["email"] = req.GetFieldByName("email")
		user["id"] = fmt.Sprintf("usr_%d", time.Now().UnixMilli())
	}
	if method.GetName() == "WatchUsers" {
		user["id"] = fmt.Sprintf("usr_%d", 1024+sequence)
	}
	switch method.GetName() {
	case "GetOrder":
		result.SetFieldByName("id", req.GetFieldByName("order_id"))
		result.SetFieldByName("status", "PAID")
		result.SetFieldByName("amount", float64(128.50))
	case "ListOrders":
		order := dynamic.NewMessage(result.FindFieldDescriptorByName("orders").GetMessageType())
		order.SetFieldByName("id", "ord_2048")
		order.SetFieldByName("status", "PAID")
		order.SetFieldByName("amount", float64(128.50))
		result.AddRepeatedFieldByName("orders", order)
	case "ListUsers":
		item := dynamic.NewMessage(result.FindFieldDescriptorByName("users").GetMessageType())
		for key, value := range user {
			item.SetFieldByName(key, value)
		}
		result.AddRepeatedFieldByName("users", item)
		result.SetFieldByName("total", int32(1))
	default:
		for key, value := range user {
			result.SetFieldByName(key, value)
		}
		include := method.GetName() != "GetUser"
		if field := req.FindFieldDescriptorByName("include_profile"); field != nil {
			include, _ = req.GetField(field).(bool)
		}
		if include {
			profile := dynamic.NewMessage(result.FindFieldDescriptorByName("profile").GetMessageType())
			profile.SetFieldByName("team", "Platform")
			profile.SetFieldByName("role", "Engineer")
			profile.SetFieldByName("region", "ap-southeast-1")
			result.SetFieldByName("profile", profile)
		}
	}
	return result, nil
}

func main() {
	address := flag.String("addr", "127.0.0.1:50051", "listen address")
	protos := flag.String("protos", "examples/protos", "proto directory")
	verifySignature := flag.Bool("verify-demo-signature", false, "require test appid + millisecond ts + MD5 signature for user/order methods")
	flag.Parse()
	verify := func(ctx context.Context) error {
		if !*verifySignature {
			return nil
		}
		headers, _ := metadata.FromIncomingContext(ctx)
		get := func(key string) string {
			values := headers.Get(key)
			if len(values) == 1 {
				return values[0]
			}
			return ""
		}
		appid, ts, sign := get("appid"), get("ts"), get("sign")
		millis, err := strconv.ParseInt(ts, 10, 64)
		if err != nil || len(ts) != 13 || appid != "rpcrad-test" || millis < time.Now().UnixMilli()-60000 || millis > time.Now().UnixMilli()+60000 {
			return status.Error(codes.Unauthenticated, "invalid appid or millisecond timestamp")
		}
		// Public test fixture only, never a production secret.
		expected := fmt.Sprintf("%x", md5.Sum([]byte("rpcrad-test-secret"+appid+ts+"rpcrad-test-secret")))
		if sign != expected {
			return status.Error(codes.Unauthenticated, "invalid signature")
		}
		return grpc.SetHeader(ctx, metadata.Pairs("x-signature-verified", "true"))
	}
	files, err := (&protoparse.Parser{ImportPaths: []string{*protos}}).ParseFiles("user.proto", "order.proto")
	if err != nil {
		log.Fatal(err)
	}
	server := grpc.NewServer()
	for _, file := range files {
		if err := protoregistry.GlobalFiles.RegisterFile(file.UnwrapFile()); err != nil {
			log.Fatal(err)
		}
		for _, service := range file.GetServices() {
			descriptor := grpc.ServiceDesc{ServiceName: service.GetFullyQualifiedName(), HandlerType: (*interface{})(nil), Metadata: file.GetName()}
			for _, method := range service.GetMethods() {
				method := method
				if method.IsServerStreaming() {
					descriptor.Streams = append(descriptor.Streams, grpc.StreamDesc{StreamName: method.GetName(), ServerStreams: true,
						Handler: func(_ interface{}, stream grpc.ServerStream) error {
							if err := verify(stream.Context()); err != nil {
								return err
							}
							req := dynamic.NewMessage(method.GetInputType())
							if err := stream.RecvMsg(req); err != nil {
								return err
							}
							stream.SendHeader(metadata.Pairs("x-server", "rpcrad-local-demo"))
							for i := 0; i < 5; i++ {
								select {
								case <-stream.Context().Done():
									return stream.Context().Err()
								case <-time.After(650 * time.Millisecond):
								}
								response, err := reply(method, req, i)
								if err != nil {
									return err
								}
								if err := stream.SendMsg(response); err != nil {
									return err
								}
							}
							stream.SetTrailer(metadata.Pairs("x-stream-complete", "true"))
							return nil
						}})
				} else {
					descriptor.Methods = append(descriptor.Methods, grpc.MethodDesc{MethodName: method.GetName(),
						Handler: func(_ interface{}, ctx context.Context, dec func(interface{}) error, _ grpc.UnaryServerInterceptor) (interface{}, error) {
							if err := verify(ctx); err != nil {
								return nil, err
							}
							req := dynamic.NewMessage(method.GetInputType())
							if err := dec(req); err != nil {
								return nil, err
							}
							select {
							case <-ctx.Done():
								return nil, ctx.Err()
							case <-time.After(42 * time.Millisecond):
							}
							grpc.SendHeader(ctx, metadata.Pairs("x-server", "rpcrad-local-demo", "x-request-id", fmt.Sprint(time.Now().UnixMilli())))
							grpc.SetTrailer(ctx, metadata.Pairs("x-rpcrad", "local"))
							return reply(method, req, 0)
						}})
				}
			}
			server.RegisterService(&descriptor, struct{}{})
		}
	}
	healthServer := health.NewServer()
	healthServer.SetServingStatus("", healthpb.HealthCheckResponse_SERVING)
	healthpb.RegisterHealthServer(server, healthServer)
	reflection.Register(server)
	listener, err := net.Listen("tcp", *address)
	if err != nil {
		log.Fatal(err)
	}
	log.Printf("RPCRad local demo listening on %s", listener.Addr())
	log.Fatal(server.Serve(listener))
}
