# opday.dev

Centralized service for [https://github.com/aptakhin/opday.dev](aptakhin/opday).

* HTTP health checks

## Manual setup

https://github.com/acmesh-official/acme.sh

```bash
mkdir -p /etc/nginx/cert

curl https://get.acme.sh | sh -s email=...
acme.sh --issue -d opday.dev -w /etc/nginx/cert
```

```bash
wget -O opday https://github.com/aptakhin/opday/releases/download/0.0.1/opday-x86_64-unknown-linux
chmod +x opday

opday build-push-deploy
```
