if config.appid == "" { throw "请填写 AppID"; }
if config.appid_header == config.timestamp_header || config.appid_header == config.signature_header || config.timestamp_header == config.signature_header { throw "三个头名称必须不同"; }
let ts = if config.timestamp_unit == "milliseconds" { request.timestamp_ms }
    else if config.timestamp_unit == "seconds" { request.timestamp }
    else { throw "时间戳单位必须为 milliseconds 或 seconds"; };
let signature = md5_secret_wrap(config.appid + ts);
if config.hex_case == "upper" { signature = signature.to_upper(); }
else if config.hex_case != "lower" { throw "MD5 大小写配置无效"; }
metadata[config.appid_header] = config.appid;
metadata[config.timestamp_header] = ts;
metadata[config.signature_header] = signature;
metadata
