# GitOps Integration

## CLI Workflow
```bash
nauto_cli gitops \
  --repo /path/to/repo \
  --inventory examples/inventory.yaml \
  --commit \
  --message "Update desired configs"
```

Steps:
1. Reads inventory YAML (devices + metadata).
2. Generates `configs/<device>.cfg` files with headers referencing mgmt addresses/tags.
3. Optionally stages + commits via libgit2 (`git2` crate).

## Repository Layout
```
repo/
 ├── configs/
 │    ├── core-r1.cfg
 │    └── agg-eos-1.cfg
 └── ...
```

## Future Enhancements
- Pull desired state from Git and feed directly into job engine.
- Support templates per vendor.
- Wire automated PR creation for compliance drifts.