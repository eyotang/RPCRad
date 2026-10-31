if config.signature_header == config.timestamp_header { throw ui_text("两个头名称必须不同", "The two header names must be different"); }
metadata[config.timestamp_header] = request.timestamp_ms;
metadata[config.signature_header] = hmac_sha256(request.method + "\n" + request.timestamp_ms + "\n" + request.body);
metadata
