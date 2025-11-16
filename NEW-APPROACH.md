# opday.dev

Deployments and ops operations made not vendor-locked.

# Easy approach (*)
* Still requires VM-machine ready, because we don't provide one (but why)?
* Spot machine maybe


```toml
echo > opday.toml << EOF
just_ssh_key_path = ""
just_hosts = [
    "root@193.184.216.449"
]
EOF
```
# Open only ports 80, 443


Execute. Every secret stays locally, not uploaded anywhere:

```bash
opday build-push-deploy
```

Extend container registry

```toml
echo > opday.toml << EOF
just_cr_credentials_path = ""
just_ssh_key_path = ""
just_hosts = [
    "root@193.184.216.449"
]
EOF
```

Declarative *with a few exclusions
Extensible.

State, diff-state, apply.




Commands can go trough service. Pull. You can click state and deploy.
Monitor.

Just manually push.

curl ".../install.sh" | sh

# TODO: add creds
opday sync
# or???
opday deploy
opday deploy --service aaa --env prod -p "opday.dev/docker.image=xxx"
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
        opday.dev/image-tag: "$NGINX_TAG"
        opday.dev/runner: "my-docker"
        opday.dev/orchestrator: "my-orchestrator"
        opday.dev/backends: "my-backends"

  backend:
    image: registry.digitalocean.com/frlr/opday-dev/backend:$BACKEND_TAG
    build: '!reset null'
    command: opday -p 3003
    restart: unless-stopped
    ports:
    - 3003:3003
    deploy:
      labels:
        opday.dev/image-tag: "$BACKEND_TAG"
        opday.dev/runner: "my-docker"
        opday.dev/orchestrator: "my-orchestrator"
        opday.dev/group: "my-group"
```



```bash
opday -c opday.toml

opday sync --plan # make a plan
opday build backend -t 277155 --push
opday deploy backend --env prod -t 277155
opday agent serve --port 26166
```


# version 333

```toml
[main]
type = "opday.dev/project"
project = "xx"
(enironment = "yy")

[my-docker]
type = "opday.dev/docker"
registry = ""
credentials_path = ""

[my-orchestrator]
type = "opday.dev/orch1"
ingress = "nginx"

[my-group]
type = "opday.dev/group"
private_key_path = ""
user_name = "opdev"

rules = [
    "vnet",
    "ports"
]

[vnet]
type = "opday.dev/ansible-vnet"
# setup vnet

[ports]
type = "opday.dev/ansible-ports"
# setup ssh
```




```toml
[opday.dev/project]
project = "xx"
(enironment = "yy")

[opday.dev/docker]
registry = ""
credentials_path = ""

[opday.dev/orch1]
ingress = "nginx"

[[opday.dev/service]]
name = "aaa"

[[opday.dev/service]]
name = "bbb"

[opday.dev/group]
private_key_path = ""
user_name = "opdev"

rules = [
    "vnet",
    "ports"
]

[vnet]
type = "opday.dev/ansible-vnet"
# setup vnet

[ports]
type = "opday.dev/ansible-ports"
# setup ssh
```



