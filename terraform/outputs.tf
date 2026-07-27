output "alb_dns_name" {
  description = "Public DNS name of the application load balancer."
  value       = module.loadbalancer.alb_dns_name
}

output "api_url" {
  description = "HTTPS URL for the PrediFi API."
  value       = "https://${var.domain_name}"
}

output "certificate_arn" {
  description = "ACM certificate ARN."
  value       = module.ssl.certificate_arn
}

output "postgres_endpoint" {
  description = "RDS PostgreSQL endpoint."
  value       = module.postgres.endpoint
  sensitive   = true
}

output "redis_primary_endpoint" {
  description = "ElastiCache Redis primary endpoint."
  value       = module.redis.primary_endpoint
  sensitive   = true
}

output "asg_name" {
  description = "Auto Scaling Group name for app compute."
  value       = module.compute.asg_name
}

output "grafana_url_hint" {
  description = "Private Grafana host (access via VPN/bastion)."
  value       = module.monitoring.grafana_private_dns
}

output "prometheus_url_hint" {
  description = "Private Prometheus host (access via VPN/bastion)."
  value       = module.monitoring.prometheus_private_dns
}
