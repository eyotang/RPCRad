if config.appid == "" { throw ui_text("请填写 AppID", "Enter an AppID"); }
if config.appid_header == config.timestamp_header || config.appid_header == config.signature_header || config.timestamp_header == config.signature_header { throw ui_text("三个头名称必须不同", "All three header names must be different"); }
let ts = if config.timestamp_unit == "milliseconds" { request.timestamp_ms }
    else if config.timestamp_unit == "seconds" { request.timestamp }
    else { throw ui_text("时间戳单位必须为 milliseconds 或 seconds", "Timestamp unit must be milliseconds or seconds"); };
let signature = md5_secret_wrap(config.appid + ts);
if config.hex_case == "upper" { signature = signature.to_upper(); }
else if config.hex_case != "lower" { throw ui_text("MD5 大小写配置无效", "Invalid MD5 letter case"); }
metadata[config.appid_header] = config.appid;
metadata[config.timestamp_header] = ts;
metadata[config.signature_header] = signature;
metadata
