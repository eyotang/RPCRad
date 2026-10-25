if !metadata.contains(config.source_header) { throw "请先在 Metadata 中添加原文头"; }
let encrypted = aes256_gcm(metadata[config.source_header], config.aad);
metadata.remove(config.source_header);
metadata[config.target_header] = encrypted;
metadata
