# Prometheus + Grafana monitoring stack on a dedicated private instance.

variable "name_prefix" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "private_subnet_ids" {
  type = list(string)
}

variable "instance_type" {
  type = string
}

variable "ami_id" {
  type = string
}

variable "key_name" {
  type    = string
  default = null
}

variable "grafana_admin_user" {
  type    = string
  default = "admin"
}

variable "allowed_cidr_blocks" {
  type        = list(string)
  description = "CIDRs allowed to reach Grafana (3000) and Prometheus (9090)."
  default     = []
}

variable "prometheus_scrape_targets" {
  type        = list(string)
  description = "Hostnames to scrape (e.g. ALB DNS). Configured into prometheus.yml via user-data."
  default     = []
}

resource "random_password" "grafana_admin" {
  length  = 24
  special = false
}

locals {
  scrape_list = join(", ", [for t in var.prometheus_scrape_targets : "\"${t}:8080\""])
  user_data = <<-EOT
    #!/bin/bash
    set -euo pipefail
    if ! command -v docker >/dev/null 2>&1; then
      curl -fsSL https://get.docker.com | sh
      systemctl enable --now docker
    fi

    mkdir -p /opt/predifi-monitoring/prometheus/alerts
    cat >/opt/predifi-monitoring/prometheus/alerts/predifi_alerts.yml <<EOF
groups:
  - name: predifi_production_alerts
    rules:
      - alert: HighApiErrorRate
        expr: (sum(rate(app_http_requests_total{status=~"5.."}[5m])) / sum(rate(app_http_requests_total[5m]))) * 100 > 1
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "API HTTP 5xx error rate exceeded 1%"
      - alert: HighP99Latency
        expr: histogram_quantile(0.99, sum(rate(app_http_request_duration_seconds_bucket[5m])) by (le)) > 0.5
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "API p99 latency is greater than 500ms"
      - alert: DatabasePoolExhausted
        expr: sum(rate(app_db_queries_total{result="error"}[5m])) > 5
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Database connection pool exhaustion or query error spike"
      - alert: HighRedisMemoryUsage
        expr: sum(rate(app_redis_operations_total{result="error"}[5m])) > 10
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High Redis operation error rate or high memory usage (>80%)"
      - alert: ContractInteractionFailures
        expr: sum(rate(app_db_queries_total{query_type=~".*contract.*",result="error"}[5m])) > 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Smart contract interaction failure detected"
      - alert: BackendMemoryHigh
        expr: (app_memory_used_bytes / app_memory_total_bytes) * 100 > 90
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Backend memory usage exceeded 90% threshold"
EOF

    cat >/opt/predifi-monitoring/prometheus/prometheus.yml <<EOF
global:
  scrape_interval: 15s
rule_files:
  - /etc/prometheus/alerts/*.yml
scrape_configs:
  - job_name: predifi
    metrics_path: /metrics
    static_configs:
      - targets: [${local.scrape_list}]
EOF

    docker rm -f prometheus grafana 2>/dev/null || true

    docker run -d --name prometheus --restart=always \
      -p 9090:9090 \
      -v /opt/predifi-monitoring/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml \
      -v /opt/predifi-monitoring/prometheus/alerts:/etc/prometheus/alerts \
      prom/prometheus:v2.54.1

    docker run -d --name grafana --restart=always \
      -p 3000:3000 \
      -e GF_SECURITY_ADMIN_USER=${var.grafana_admin_user} \
      -e GF_SECURITY_ADMIN_PASSWORD=${random_password.grafana_admin.result} \
      grafana/grafana:11.2.0
  EOT
}

resource "aws_security_group" "monitoring" {
  name_prefix = "${var.name_prefix}-mon-"
  description = "PrediFi Prometheus + Grafana"
  vpc_id      = var.vpc_id

  dynamic "ingress" {
    for_each = length(var.allowed_cidr_blocks) > 0 ? [1] : []
    content {
      description = "Grafana UI"
      from_port   = 3000
      to_port     = 3000
      protocol    = "tcp"
      cidr_blocks = var.allowed_cidr_blocks
    }
  }

  dynamic "ingress" {
    for_each = length(var.allowed_cidr_blocks) > 0 ? [1] : []
    content {
      description = "Prometheus UI"
      from_port   = 9090
      to_port     = 9090
      protocol    = "tcp"
      cidr_blocks = var.allowed_cidr_blocks
    }
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.name_prefix}-monitoring-sg"
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_instance" "monitoring" {
  ami                    = var.ami_id
  instance_type          = var.instance_type
  subnet_id              = var.private_subnet_ids[0]
  vpc_security_group_ids = [aws_security_group.monitoring.id]
  key_name               = var.key_name
  user_data              = local.user_data

  root_block_device {
    volume_size = 40
    encrypted   = true
  }

  tags = {
    Name = "${var.name_prefix}-monitoring"
  }
}

output "instance_id" {
  value = aws_instance.monitoring.id
}

output "grafana_private_dns" {
  value = aws_instance.monitoring.private_dns
}

output "prometheus_private_dns" {
  value = aws_instance.monitoring.private_dns
}

output "grafana_admin_password" {
  value     = random_password.grafana_admin.result
  sensitive = true
}

output "security_group_id" {
  value = aws_security_group.monitoring.id
}
