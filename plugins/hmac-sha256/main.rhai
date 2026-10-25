if config.signature_header == config.timestamp_header { throw "两个头名称必须不同"; }
metadata[config.timestamp_header] = request.timestamp_ms;
metadata[config.signature_header] = hmac_sha256(request.method + "\n" + request.timestamp_ms + "\n" + request.body);
metadata
