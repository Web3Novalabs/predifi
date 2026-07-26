# Production environment wrapper — set real VPC/subnet/AMI values in terraform.tfvars.

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

  # Uncomment and configure for remote state:
  # backend "s3" {
  #   bucket = "predifi-terraform-state"
  #   key    = "production/terraform.tfstate"
  #   region = "us-east-1"
  # }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "predifi"
      Environment = "production"
      ManagedBy   = "terraform"
    }
  }
}

module "predifi" {
  source = "../.."

  environment = "production"
  aws_region  = var.aws_region

  vpc_id             = var.vpc_id
  public_subnet_ids  = var.public_subnet_ids
  private_subnet_ids = var.private_subnet_ids
  ami_id             = var.ami_id
  key_name           = var.key_name

  domain_name     = var.domain_name
  route53_zone_id = var.route53_zone_id
  ssl_sans        = var.ssl_sans

  compute_instance_type    = var.compute_instance_type
  compute_desired_capacity = var.compute_desired_capacity
  db_instance_class        = var.db_instance_class
  db_multi_az              = true
  redis_num_nodes          = 2

  monitoring_allowed_cidrs = var.monitoring_allowed_cidrs
}

variable "aws_region" {
  type    = string
  default = "us-east-1"
}

variable "vpc_id" {
  type = string
}

variable "public_subnet_ids" {
  type = list(string)
}

variable "private_subnet_ids" {
  type = list(string)
}

variable "ami_id" {
  type = string
}

variable "key_name" {
  type    = string
  default = null
}

variable "domain_name" {
  type = string
}

variable "route53_zone_id" {
  type = string
}

variable "ssl_sans" {
  type    = list(string)
  default = []
}

variable "compute_instance_type" {
  type    = string
  default = "t3.medium"
}

variable "compute_desired_capacity" {
  type    = number
  default = 2
}

variable "db_instance_class" {
  type    = string
  default = "db.t4g.medium"
}

variable "monitoring_allowed_cidrs" {
  type    = list(string)
  default = []
}

output "api_url" {
  value = module.predifi.api_url
}

output "alb_dns_name" {
  value = module.predifi.alb_dns_name
}

output "postgres_endpoint" {
  value     = module.predifi.postgres_endpoint
  sensitive = true
}

output "redis_primary_endpoint" {
  value     = module.predifi.redis_primary_endpoint
  sensitive = true
}
