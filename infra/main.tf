# ABOUTME: Orchestrates all Terraform modules for Pierre MCP Server infrastructure
# ABOUTME: Manages dependencies between modules with explicit depends_on

# -----------------------------------------------------------------------------
# Project APIs (must be first)
# -----------------------------------------------------------------------------

module "project" {
  source = "./modules/project"

  project_id = var.project_id
}

# -----------------------------------------------------------------------------
# Networking (depends on APIs)
# -----------------------------------------------------------------------------

module "networking" {
  source = "./modules/networking"

  project_id         = var.project_id
  region             = var.region
  vpc_name           = var.vpc_name
  subnet_cidr        = var.subnet_cidr
  vpc_connector_cidr = var.vpc_connector_cidr

  depends_on = [module.project]
}

# -----------------------------------------------------------------------------
# Secrets (depends on APIs)
# -----------------------------------------------------------------------------

module "secrets" {
  source = "./modules/secrets"

  project_id   = var.project_id
  service_name = var.service_name
  labels       = var.labels

  depends_on = [module.project]
}

# -----------------------------------------------------------------------------
# Database (depends on networking and secrets)
# -----------------------------------------------------------------------------

module "database" {
  source = "./modules/database"

  project_id                = var.project_id
  region                    = var.region
  service_name              = var.service_name
  environment               = var.environment
  vpc_self_link             = module.networking.vpc_self_link
  private_vpc_connection_id = module.networking.private_vpc_connection_id
  database_version          = var.database_version
  database_tier             = var.database_tier
  database_name             = var.database_name
  database_user             = var.database_user
  database_password         = module.secrets.db_password
  deletion_protection       = var.database_deletion_protection
  backup_enabled            = var.database_backup_enabled
  backup_start_time         = var.database_backup_start_time
  labels                    = var.labels

  depends_on = [module.networking, module.secrets]
}

# -----------------------------------------------------------------------------
# Artifact Registry (depends on APIs)
# -----------------------------------------------------------------------------

module "artifact_registry" {
  source = "./modules/artifact_registry"

  project_id    = var.project_id
  region        = var.region
  registry_name = var.registry_name
  labels        = var.labels

  depends_on = [module.project]
}

# -----------------------------------------------------------------------------
# Service Accounts (depends on APIs)
# -----------------------------------------------------------------------------

module "service_accounts" {
  source = "./modules/service_accounts"

  project_id   = var.project_id
  service_name = var.service_name

  depends_on = [module.project]
}

# -----------------------------------------------------------------------------
# Workload Identity (depends on service accounts)
# -----------------------------------------------------------------------------

module "workload_identity" {
  source = "./modules/workload_identity"

  project_id                    = var.project_id
  github_org                    = var.github_org
  github_repo                   = var.github_repo
  deployer_service_account_name = module.service_accounts.deployer_service_account_name

  depends_on = [module.service_accounts]
}

# -----------------------------------------------------------------------------
# Storage (optional, depends on APIs)
# -----------------------------------------------------------------------------

module "storage" {
  source = "./modules/storage"

  project_id                    = var.project_id
  region                        = var.region
  service_name                  = var.service_name
  create_app_bucket             = false
  create_terraform_state_bucket = false
  labels                        = var.labels

  depends_on = [module.project]
}
