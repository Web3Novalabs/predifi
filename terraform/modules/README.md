# Terraform Infrastructure Modules

This directory contains reusable Terraform modules used to provision and configure the PrediFi platform infrastructure on AWS.

## Architecture Overview

PrediFi runs on a modular AWS architecture:
- **DNS & SSL**: Route53 handles domain routing and automated ACM TLS certificate validation.
- **Load Balancing & Ingress**: An Application Load Balancer (ALB) handles public HTTPS termination, HTTP-to-HTTPS redirect, and forwards traffic to the backend instances.
- **Compute**: An Auto Scaling Group (ASG) running the Axum backend application instances in private VPC subnets.
- **Databases & State**: Amazon RDS PostgreSQL for persistent relational data and Amazon ElastiCache Redis for caching, sessions, and realtime subscriptions.
- **Observability**: A standalone monitoring EC2 instance running Prometheus and Grafana with pre-baked alerting rules.

```
       Internet
          │
          ▼
   [ Route53 (DNS) ] ── (ACM SSL Cert)
          │
          ▼
    [ Public ALB ]
          │ (Port 8080)
    ┌─────┴─────────────────────────┐
    │ Private Subnets               │
    │  ┌─────────────────────────┐  │
    │  │  Compute ASG (Axum API) │  │
    │  └──────┬───────────┬──────┘  │
    │         │           │         │
    │         ▼           ▼         │
    │  [ RDS Postgres ] [ Redis ]   │
    │                               │
    │  ┌─────────────────────────┐  │
    │  │ Monitoring (Prom/Graf)  │  │
    │  └─────────────────────────┘  │
    └───────────────────────────────┘
```

---

## Modules Summary

| Module | Purpose & Provisioned Resources | Required Inputs | Optional Inputs | Key Outputs |
| :--- | :--- | :--- | :--- | :--- |
| [`compute`](#1-compute) | Auto Scaling Group (ASG), Launch Template, Security Group for Axum backend instances behind ALB | `name_prefix`<br>`vpc_id`<br>`private_subnet_ids`<br>`instance_type`<br>`desired_capacity`<br>`min_size`<br>`max_size`<br>`ami_id`<br>`target_group_arns` | `key_name` (`null`)<br>`user_data` (`""`) | `security_group_id`<br>`asg_name`<br>`launch_template_id` |
| [`dns`](#2-dns) | Route53 Alias A records pointing apex and optional `www` subdomain to ALB | `route53_zone_id`<br>`domain_name`<br>`alb_dns_name`<br>`alb_zone_id` | `create_www_record` (`false`) | `fqdn` |
| [`loadbalancer`](#3-loadbalancer) | Public Application Load Balancer (ALB), Target Group, HTTPS listener (443), and HTTP-to-HTTPS redirect listener (80) | `name_prefix`<br>`vpc_id`<br>`public_subnet_ids`<br>`certificate_arn` | `health_check_path` (`"/health"`)<br>`app_port` (`8080`) | `alb_dns_name`<br>`alb_zone_id`<br>`alb_arn`<br>`api_target_group_arn`<br>`security_group_id` |
| [`monitoring`](#4-monitoring) | Dedicated EC2 instance running Prometheus (v2.54.1) & Grafana (11.2.0) containers via Docker with preloaded alert rules | `name_prefix`<br>`vpc_id`<br>`private_subnet_ids`<br>`instance_type`<br>`ami_id` | `key_name` (`null`)<br>`grafana_admin_user` (`"admin"`)<br>`allowed_cidr_blocks` (`[]`)<br>`prometheus_scrape_targets` (`[]`) | `instance_id`<br>`grafana_private_dns`<br>`prometheus_private_dns`<br>`grafana_admin_password`<br>`security_group_id` |
| [`postgres`](#5-postgres) | Amazon RDS PostgreSQL instance with automated backups, deletion protection, and app security group rules | `name_prefix`<br>`vpc_id`<br>`private_subnet_ids`<br>`db_name`<br>`db_username`<br>`engine_version`<br>`instance_class`<br>`allocated_storage`<br>`multi_az`<br>`allowed_security_group` | *(None)* | `endpoint`<br>`port`<br>`db_name`<br>`master_password`<br>`security_group_id` |
| [`redis`](#6-redis) | Amazon ElastiCache Redis replication group with transit & at-rest encryption and auto-failover | `name_prefix`<br>`vpc_id`<br>`private_subnet_ids`<br>`node_type`<br>`num_cache_clusters`<br>`engine_version`<br>`allowed_security_group` | *(None)* | `primary_endpoint`<br>`reader_endpoint`<br>`port`<br>`security_group_id` |
| [`ssl`](#7-ssl) | AWS Certificate Manager (ACM) TLS certificate with automated Route53 DNS validation | `domain_name`<br>`route53_zone_id` | `subject_alternative_names` (`[]`) | `certificate_arn`<br>`domain_name` |

---

## Detailed Module Reference

### 1. Compute

**Path:** `terraform/modules/compute/`

Provisions an EC2 Launch Template and Auto Scaling Group (ASG) for running the PrediFi Axum backend. Application instances reside in private subnets and accept inbound TCP traffic on port `8080` originating from the VPC (`10.0.0.0/8`).

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name_prefix` | `string` | *required* | Resource naming prefix |
| `vpc_id` | `string` | *required* | AWS VPC ID |
| `private_subnet_ids` | `list(string)` | *required* | Subnet IDs for EC2 placement |
| `instance_type` | `string` | *required* | EC2 instance size (e.g. `t3.medium`) |
| `desired_capacity` | `number` | *required* | Desired number of EC2 instances |
| `min_size` | `number` | *required* | Minimum ASG instance count |
| `max_size` | `number` | *required* | Maximum ASG instance count |
| `ami_id` | `string` | *required* | AMI ID for launching instances |
| `target_group_arns` | `list(string)` | *required* | ALB target group ARNs to register instances with |
| `key_name` | `string` | `null` | Optional SSH key pair name |
| `user_data` | `string` | `""` | User-data script executed at instance boot |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `security_group_id` | `string` | ID of the application security group |
| `asg_name` | `string` | Name of the provisioned Auto Scaling Group |
| `launch_template_id` | `string` | ID of the EC2 launch template |

---

### 2. DNS

**Path:** `terraform/modules/dns/`

Creates Route53 alias records (`A`) that route traffic from apex domains and optional `www` subdomains to the Application Load Balancer.

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `route53_zone_id` | `string` | *required* | Route53 hosted zone ID |
| `domain_name` | `string` | *required* | Apex domain name (e.g. `predifi.com`) |
| `alb_dns_name` | `string` | *required* | DNS hostname of the Application Load Balancer |
| `alb_zone_id` | `string` | *required* | Hosted zone ID of the Application Load Balancer |
| `create_www_record` | `bool` | `false` | Whether to create a `www.${domain_name}` record |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `fqdn` | `string` | Fully qualified domain name of the apex record |

---

### 3. Load Balancer

**Path:** `terraform/modules/loadbalancer/`

Provisions a public Application Load Balancer (ALB) across public subnets with listeners for port 80 (HTTP 301 redirect to HTTPS) and port 443 (HTTPS forwarding to the backend target group).

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name_prefix` | `string` | *required* | Resource naming prefix |
| `vpc_id` | `string` | *required* | AWS VPC ID |
| `public_subnet_ids` | `list(string)` | *required* | Public subnet IDs for ALB placement |
| `certificate_arn` | `string` | *required* | ACM SSL certificate ARN for HTTPS listener |
| `health_check_path` | `string` | `"/health"` | Backend HTTP path for ALB health checks |
| `app_port` | `number` | `8080` | Port the backend service listens on |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `alb_dns_name` | `string` | Public DNS name of the ALB |
| `alb_zone_id` | `string` | Route53 zone ID of the ALB |
| `alb_arn` | `string` | ARN of the Application Load Balancer |
| `api_target_group_arn` | `string` | ARN of the ALB target group |
| `security_group_id` | `string` | Security group ID of the ALB |

---

### 4. Monitoring

**Path:** `terraform/modules/monitoring/`

Provisions a dedicated monitoring EC2 instance with Docker hosting Prometheus and Grafana. Configured with alerting rules for API 5xx errors, P99 latency thresholds (>500ms), DB connection pool exhaustion, high Redis memory usage, smart contract interaction errors, and high host memory usage.

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name_prefix` | `string` | *required* | Resource naming prefix |
| `vpc_id` | `string` | *required* | AWS VPC ID |
| `private_subnet_ids` | `list(string)` | *required* | Subnets for monitoring instance |
| `instance_type` | `string` | *required* | EC2 instance type |
| `ami_id` | `string` | *required* | AMI ID for the EC2 instance |
| `key_name` | `string` | `null` | Optional SSH key pair name |
| `grafana_admin_user` | `string` | `"admin"` | Username for Grafana web console |
| `allowed_cidr_blocks` | `list(string)` | `[]` | Allowed CIDR blocks for accessing Grafana (3000) and Prometheus (9090) |
| `prometheus_scrape_targets` | `list(string)` | `[]` | Targets scraped by Prometheus on port 8080 |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `instance_id` | `string` | EC2 Instance ID of the monitoring host |
| `grafana_private_dns` | `string` | Private DNS name for Grafana access |
| `prometheus_private_dns` | `string` | Private DNS name for Prometheus access |
| `grafana_admin_password` | `string` *(sensitive)* | Auto-generated administrator password for Grafana |
| `security_group_id` | `string` | Security group ID of the monitoring instance |

---

### 5. PostgreSQL

**Path:** `terraform/modules/postgres/`

Provisions an Amazon RDS PostgreSQL database in a dedicated DB subnet group with encrypted storage, multi-AZ options, automated 7-day backups, performance insights, and access restricted to the application security group on port 5432.

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name_prefix` | `string` | *required* | Resource naming prefix |
| `vpc_id` | `string` | *required* | AWS VPC ID |
| `private_subnet_ids` | `list(string)` | *required* | Subnet IDs for RDS subnet group |
| `db_name` | `string` | *required* | Name of the PostgreSQL database to create |
| `db_username` | `string` | *required* | Master username |
| `engine_version` | `string` | *required* | PostgreSQL version (e.g. `"15.4"`) |
| `instance_class` | `string` | *required* | RDS DB instance class (e.g. `"db.t3.medium"`) |
| `allocated_storage` | `number` | *required* | Initial storage in GB |
| `multi_az` | `bool` | *required* | Enable multi-AZ deployment for high availability |
| `allowed_security_group` | `string` | *required* | Security group ID of compute instances permitted to connect on port 5432 |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `endpoint` | `string` *(sensitive)* | RDS database connection host/endpoint |
| `port` | `number` | Port for PostgreSQL connections (`5432`) |
| `db_name` | `string` | Database name |
| `master_password` | `string` *(sensitive)* | Auto-generated master password |
| `security_group_id` | `string` | Security group ID of the RDS instance |

---

### 6. Redis

**Path:** `terraform/modules/redis/`

Provisions an Amazon ElastiCache Redis replication group with at-rest encryption, in-transit encryption, and automatic failover across private subnets.

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name_prefix` | `string` | *required* | Resource naming prefix |
| `vpc_id` | `string` | *required* | AWS VPC ID |
| `private_subnet_ids` | `list(string)` | *required* | Subnet IDs for ElastiCache subnet group |
| `node_type` | `string` | *required* | ElastiCache node type (e.g. `"cache.t3.micro"`) |
| `num_cache_clusters` | `number` | *required* | Number of cache clusters in the replication group (>1 enables multi-AZ failover) |
| `engine_version` | `string` | *required* | Redis version (e.g. `"7.0"`) |
| `allowed_security_group` | `string` | *required* | Security group ID of compute instances permitted to connect on port 6379 |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `primary_endpoint` | `string` *(sensitive)* | Address of the primary Redis node for reads and writes |
| `reader_endpoint` | `string` *(sensitive)* | Address of the reader endpoint for load-balanced reads |
| `port` | `number` | Port number (`6379`) |
| `security_group_id` | `string` | Security group ID of the Redis cluster |

---

### 7. SSL

**Path:** `terraform/modules/ssl/`

Requests a public TLS certificate from AWS Certificate Manager (ACM) and creates Route53 DNS validation records in the specified hosted zone.

#### Inputs
| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `domain_name` | `string` | *required* | Primary domain name (e.g. `"predifi.com"`) |
| `route53_zone_id` | `string` | *required* | Route53 hosted zone ID for DNS validation records |
| `subject_alternative_names` | `list(string)` | `[]` | Additional SAN domains (e.g. `["*.predifi.com"]`) |

#### Outputs
| Output | Type | Description |
| :--- | :--- | :--- |
| `certificate_arn` | `string` | ARN of the validated ACM TLS certificate |
| `domain_name` | `string` | Primary domain name on the certificate |
