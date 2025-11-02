Declarative *with a few exclusions
Extensible.

State, diff-state, apply.


Commands can go trough service. Pull. You can click state and deploy.
Monitor.



Just manually push.


[dev.opday.project]
project = "xx"

runner = "dev.opday.docker"
orchestrator = "dev.opday.orch1"

[dev.opday]
plugins = [
    "fbff"
]

pull = true
host = "https://opday.cloud"
env_file = [
    ".env",
]

[[dev.opday.service]]
name = "aaa.prod"
service = "backend"  ## from docker compose
hosts = [
    "root@aaa"
]

instances = [
    "aaa:8000"
]


[dev.opday.orch1]
ingress = "nginx"
services = [
    "nginx",
]
params = [
    "dev.opday.orch1.backends=aaa.prod",
]



Or


[dev.opday.project]
project = "xx"
(enironment = "yy")

runner = "dev.opday.docker"
orchestrator = "dev.opday.orch1"


state_path = "file"

[dev.opday.agent]
host = "https://xxx"
data_dir = "" # fdb data

[dev.opday.runner]
registry = ""
credentials_path = ""

[dev.opday.orch1]
ingress = "nginx"

[group]
type = "group"
private_key_path = ""
user_name = "opdev"

[group.aaa-dev]
type = "group"
hosts = [
    "aaa-dev"
]

[group.aaa-prod]
type = "group"
hosts = [
    "aaa-prod1",
    "aaa-prod2",
]

[aaa]
type = "service"
name = "aaa"

[aaa.dev]
env = "dev"
group = "group.aaa-dev"

instances = [
    "aaa-dev:8000",
]

[aaa.prod]
env = "prod"
group = "group.aaa-prod"
params = [
    "dev.opday.docker.image=xxx",
]
instances = [
    "aaa-prod1:8000",
    "aaa-prod1:8001",
    "aaa-prod2:8001",
]
curl ".../install.sh" | sh

# TODO: add creds
opday plan
opday sync
# or???
opday deploy
opday deploy --service aaa --env prod -p "dev.opday.docker.image=xxx"
.env:
OPDAY_BASIC_AUTH="dfdfdff"


Docker compose prod:

```yaml
services:
  nginx:
    image: registry.digitalocean.com/frlr/opday-dev/nginx:$NGINX_TAG
    volumes:
    # pushed to image
    # - ./nginx/nginx.conf:/etc/nginx/nginx.conf
    # - ./nginx/wwwroot/:/etc/nginx/html
    - type: bind
      source: /root/.acme.sh/opday.dev_ecc/
      target: /root/.acme.sh/opday.dev_ecc/
      read_only: true
    - type: bind
      source: /etc/nginx/cert
      target: /etc/nginx/cert
      read_only: true
    ports:
    - "80:80"
    - "443:443"
    deploy:
      labels:
        dev.opday.image-tag: "$NGINX_TAG"
        dev.opday.runner: "dev.opday.docker"
        dev.opday.orchestrator: "dev.opday.orch1"
        dev.opday.backends.main: "opday-backends"

  backend:
    image: registry.digitalocean.com/frlr/opday-dev/backend:$BACKEND_TAG
    build: '!reset null'
    command: opday-dev -p 3003
    restart: unless-stopped
    ports:
    - 3003:3003
    deploy:
      labels:
        dev.opday.image-tag: "$BACKEND_TAG"
        dev.opday.runner: "dev.opday.docker"
        dev.opday.orchestrator: "dev.opday.orch1"
        dev.opday.group: "opday-backends"
```