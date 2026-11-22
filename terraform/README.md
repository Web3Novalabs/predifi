# PrediFi Infrastructure as Code

Terraform modules for infrastructure provisioning on AWS across staging and production environments.

## Layout

```
terraform/
  main.tf                 # Root module wiring
  variables.tf
  outputs.tf
  modules/
    compute/              # ASG + launch template for API instances
    postgres/             # RDS PostgreSQL (Multi-AZ capable)
    redis/                # ElastiCache Redis replication group
    loadbalancer/         # ALB + HTTPS listener + target group
    ssl/                  # ACM certificate + Route53 DNS validation
    dns/                  # Apex (and optional www) alias to ALB
    monitoring/           # Prometheus + Grafana on a private instance
  environments/
    staging/              # Cost-optimized staging env wrapper + example tfvars
    production/           # High-availability production env wrapper + example tfvars
```

## Quick start

### Deploying to Staging

```bash
cd terraform/environments/staging
cp terraform.tfvars.example terraform.tfvars
# edit terraform.tfvars with real staging VPC / subnet / AMI / domain values
terraform init
terraform plan
terraform apply
```

### Deploying to Production

```bash
cd terraform/environments/production
cp terraform.tfvars.example terraform.tfvars
# edit terraform.tfvars with real production VPC / subnet / AMI / domain values
terraform init
terraform plan
terraform apply
```

## Environment Sizing Comparison

| Parameter | Staging | Production | Rationale |
|-----------|---------|------------|-----------|
| `compute_instance_type` | `t3.small` (2 vCPU, 2 GiB) | `t3.medium` (2 vCPU, 4 GiB) | Staging handles lighter test workloads; production requires headroom for production API traffic. |
| `compute_desired_capacity` | `1` | `2` | Staging runs a single instance for cost efficiency; production requires multi-instance high availability. |
| `db_instance_class` | `db.t4g.small` (2 vCPU, 2 GiB) | `db.t4g.medium` (2 vCPU, 4 GiB) | Staging uses smaller Graviton instance class; production handles high transaction volume. |
| `db_multi_az` | `false` | `true` | Multi-AZ standby failover is enabled in production for 99.95% availability; disabled in staging to reduce costs. |
| `redis_num_nodes` | `1` | `2` | Single-node ElastiCache cache in staging; primary + replica replication group in production. |

## Modules covered

| Module | Resources |
|--------|-----------|
| compute | Security group, launch template, Auto Scaling Group |
| postgres | Subnet group, SG, RDS instance, generated master password |
| redis | Subnet group, SG, replication group (TLS + at-rest encryption) |
| loadbalancer | ALB, HTTPS/HTTP listeners, API target group |
| ssl | ACM cert + DNS validation records |
| dns | Route53 A/alias records |
| monitoring | EC2 with Dockerized Prometheus v2.54 + Grafana 11 |

## Notes

- App instances listen on `:8080` and are only reachable from the VPC / ALB path.
- Grafana admin password is generated and exposed as a sensitive output.
- Configure an S3 backend in `environments/staging/main.tf` or `environments/production/main.tf` before team use.
