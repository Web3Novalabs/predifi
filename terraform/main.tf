# PrediFi production root — wires reusable modules for compute, data, edge, and observability.
#
# Prefer applying via `environments/production` so providers/backends live at the
# environment layer. This file is a reusable composition module.

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
  }
}

data "aws_availability_zones" "available" {
  state = "available"
}

locals {
  azs         = slice(data.aws_availability_zones.available.names, 0, 2)
  name_prefix = "${var.project}-${var.environment}"
}

module "compute" {
  source = "../../modules/compute"

  name_prefix        = local.name_prefix
  vpc_id             = var.vpc_id
  private_subnet_ids = var.private_subnet_ids
  instance_type      = var.compute_instance_type
  desired_capacity   = var.compute_desired_capacity
  min_size           = var.compute_min_size
  max_size           = var.compute_max_size
  ami_id             = var.ami_id
  key_name           = var.key_name
  user_data          = var.compute_user_data
  target_group_arns  = [module.loadbalancer.api_target_group_arn]
}

module "postgres" {
  source = "../../modules/postgres"

  name_prefix            = local.name_prefix
  vpc_id                 = var.vpc_id
  private_subnet_ids     = var.private_subnet_ids
  db_name                = var.db_name
  db_username            = var.db_username
  engine_version         = var.db_engine_version
  instance_class         = var.db_instance_class
  allocated_storage      = var.db_allocated_storage
  multi_az               = var.db_multi_az
  allowed_security_group = module.compute.security_group_id
}

module "redis" {
  source = "../../modules/redis"

  name_prefix            = local.name_prefix
  vpc_id                 = var.vpc_id
  private_subnet_ids     = var.private_subnet_ids
  node_type              = var.redis_node_type
  num_cache_clusters     = var.redis_num_nodes
  engine_version         = var.redis_engine_version
  allowed_security_group = module.compute.security_group_id
}

module "ssl" {
  source = "../../modules/ssl"

  domain_name     = var.domain_name
  route53_zone_id = var.route53_zone_id
  subject_alternative_names = var.ssl_sans
}

module "loadbalancer" {
  source = "../../modules/loadbalancer"

  name_prefix        = local.name_prefix
  vpc_id             = var.vpc_id
  public_subnet_ids  = var.public_subnet_ids
  certificate_arn    = module.ssl.certificate_arn
  health_check_path  = var.health_check_path
}

module "dns" {
  source = "../../modules/dns"

  route53_zone_id   = var.route53_zone_id
  domain_name       = var.domain_name
  alb_dns_name      = module.loadbalancer.alb_dns_name
  alb_zone_id       = module.loadbalancer.alb_zone_id
  create_www_record = var.create_www_record
}

module "monitoring" {
  source = "../../modules/monitoring"

  name_prefix        = local.name_prefix
  vpc_id             = var.vpc_id
  private_subnet_ids = var.private_subnet_ids
  instance_type      = var.monitoring_instance_type
  ami_id             = var.ami_id
  key_name           = var.key_name
  grafana_admin_user = var.grafana_admin_user
  allowed_cidr_blocks = var.monitoring_allowed_cidrs
  prometheus_scrape_targets = [
    module.loadbalancer.alb_dns_name,
  ]
}
