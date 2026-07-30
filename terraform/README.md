# PrediFi Infrastructure as Code

Terraform modules for production provisioning on AWS.

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
    production/           # Env wrapper + example tfvars
```

## Quick start

```bash
cd terraform/environments/production
cp terraform.tfvars.example terraform.tfvars
# edit terraform.tfvars with real VPC / subnet / AMI / domain values
terraform init
terraform plan
terraform apply
```

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
- Configure an S3 backend in `environments/production/main.tf` before team use.
