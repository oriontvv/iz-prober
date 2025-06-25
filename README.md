# iz-prober

[iz-prober](https://github.com/oriontvv/iz-prober) is a simple telegram bot - prober in rust that checks servers availability.

# Installation
1. install binary
    * download [built](https://github.com/oriontvv/iz-prober/releases)
    * install with cargo-binstall
    `cargo install cargo-binstall && cargo binstall iz-prober`
2. create new bot with [botfather](https://t.me/BotFather) with `/newbot`, get token
3. describe `config.toml` like
```toml
check_interval_seconds = 60
telegram_token = "topsecrettoken"
telegram_chat_id = 123456789
failure_threshold = 3
servers = [
    "http://example.com",
    "http://example.org",
    "http://example.net",
]
```

4. systemd unit (optional)

* 
