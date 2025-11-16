# opday.dev

Deployments and ops operations made not vendor-locked.

# Easy approach (*)
* Still requires VM-machine ready, because we don't provide one


```toml
echo > opday.toml << EOF
# Uses current default id_rsa key to access the VM
# Opens only ports 80, 443
just_hosts = [
    "root@193.184.216.449"
]
EOF

echo > docker-compose.yaml << EOF
services:
  nginx:
    image: nginx:latest
    ports:
    - "80:80"
EOF
```

Execute. Every secret stays locally, not uploaded anywhere:

```bash
opday sync
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
