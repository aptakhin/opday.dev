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

## Deploy

```bash
# From linux
wget -O opday https://github.com/aptakhin/opday/releases/download/0.0.1/opday-x86_64-unknown-linux
chmod +x opday

./opday docker build-push-deploy --build-arg BACKEND_TAG=0.0.1

# get refinery_cli https://github.com/rust-db/refinery
wget https://github.com/rust-db/refinery/releases/download/0.8.14/refinery-0.8.14-x86_64-apple-darwin.tar.gz
```


## Dev

```bash
docker compose up -d postgres

export DATABASE_URI=postgresql://postgres:postgres@localhost:5432/postgres
refinery migrate -e DATABASE_URI -p ./opday-dev/migrations
```
