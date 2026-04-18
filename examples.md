# TLD CLI Examples

These commands use only local workspace files (`elements.yaml` and `connectors.yaml`) so they are safe to run offline.

```bash
tld init
tld add Backend --ref backend --kind service --tag layer:domain
tld add Api --parent backend --technology Go --tag protocol:rest
tld add Database --parent backend --technology PostgreSQL --tag role:database
tld connect api database --view backend --label reads-writes --relationship uses
tld update element api description Public_HTTP_API
tld update connector backend:api:database:reads-writes direction both
tld views
tld validate --skip-symbols
tld check
```

You can remove resources with:

```bash
tld remove connector --view backend --from api --to database
tld remove element database
```
