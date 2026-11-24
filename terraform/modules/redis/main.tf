# Redis (ElastiCache replication group) for PrediFi cache / sessions.

variable "name_prefix" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "private_subnet_ids" {
  type = list(string)
}

variable "node_type" {
  type = string
}

variable "num_cache_clusters" {
  type = number
}

variable "engine_version" {
  type = string
}

variable "allowed_security_group" {
  type = string
}

resource "aws_elasticache_subnet_group" "this" {
  name       = "${var.name_prefix}-redis"
  subnet_ids = var.private_subnet_ids
}

resource "aws_security_group" "redis" {
  name_prefix = "${var.name_prefix}-redis-"
  description = "PrediFi Redis cluster"
  vpc_id      = var.vpc_id

  ingress {
    description     = "Redis from app"
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [var.allowed_security_group]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${var.name_prefix}-redis-sg"
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_elasticache_replication_group" "this" {
  replication_group_id = substr("${var.name_prefix}-redis", 0, 40)
  description          = "PrediFi Redis"
  engine               = "redis"
  engine_version       = var.engine_version
  node_type            = var.node_type
  num_cache_clusters   = var.num_cache_clusters
  port                 = 6379

  subnet_group_name  = aws_elasticache_subnet_group.this.name
  security_group_ids = [aws_security_group.redis.id]

  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  automatic_failover_enabled = var.num_cache_clusters > 1
  multi_az_enabled           = var.num_cache_clusters > 1

  tags = {
    Name = "${var.name_prefix}-redis"
  }
}

output "primary_endpoint" {
  value     = aws_elasticache_replication_group.this.primary_endpoint_address
  sensitive = true
}

output "reader_endpoint" {
  value     = aws_elasticache_replication_group.this.reader_endpoint_address
  sensitive = true
}

output "port" {
  value = 6379
}

output "security_group_id" {
  value = aws_security_group.redis.id
}
