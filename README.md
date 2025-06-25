# iz-prober

[iz-prober](https://github.com/oriontvv/iz-prober) is a simple telegram bot - prober in rust that checks servers availability.

[![Actions Status](https://github.com/oriontvv/iz-prober/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/oriontvv/iz-prober/actions/workflows/ci.yml) [![Coverage badge](https://raw.githubusercontent.com/oriontvv/iz-prober/coverage/htmlcov/badges/flat.svg)](https://htmlpreview.github.io/?https://github.com/oriontvv/iz-prober/coverage/htmlcov/index.html) [![dependency status](https://deps.rs/repo/github/oriontvv/iz-prober/status.svg)](https://deps.rs/repo/github/oriontvv/iz-prober) [![Crates.io](https://img.shields.io/crates/v/iz-prober.svg)](https://crates.io/crates/iz-prober)

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
