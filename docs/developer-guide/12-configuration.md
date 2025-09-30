# Configuration System

## Overview

Pierre MCP Server uses a layered configuration system that combines environment variables, configuration files, and runtime settings to provide flexible deployment options across development, staging, and production environments.

## Configuration Architecture

```mermaid
graph TB
    subgraph "Configuration Sources"
        EnvVars[Environment Variables]
        ConfigFiles[Configuration Files]
        Defaults[Built-in Defaults]
        Runtime[Runtime Settings]
    end
    
    subgraph "Configuration Layers"
        EnvConfig[Environment Config]
        FitnessConfig[Fitness Config]
        ServerConfig[Server Config]
        DatabaseConfig[Database Config]
        OAuthConfig[OAuth Config]
    end
    
    subgraph "Configuration Management"
        Loader[Config Loader]
        Validator[Config Validator]
        Merger[Config Merger]
        Parser[Parser]
    end
    
    subgraph "Application Components"
        Server[Server Instance]
        Database[Database Layer]
        Auth[Auth Manager]
        Providers[Provider Factory]
    end
    
    EnvVars --> Loader
    ConfigFiles --> Loader
    Defaults --> Loader
    Runtime --> Loader
    
    Loader --> Parser
    Parser --> Validator
    Validator --> Merger
    
    Merger --> EnvConfig
    Merger --> FitnessConfig
    Merger --> ServerConfig
    Merger --> DatabaseConfig
    Merger --> OAuthConfig
    
    EnvConfig --> Server
    ServerConfig --> Server
    DatabaseConfig --> Database
    OAuthConfig --> Auth
    FitnessConfig --> Providers
```

## Configuration Hierarchy

Configuration values are resolved in the following order (highest priority first):

1. **Runtime Settings** - Dynamic configuration changes
2. **Environment Variables** - Deployment-specific overrides
3. **Configuration Files** - Static configuration files
4. **Built-in Defaults** - Fallback values

## Environment Configuration

### Core Environment Variables

```bash
# === Server Configuration ===
# From src/constants/mod.rs:32-173
HTTP_PORT=8081                         # Single unified port (src/constants/mod.rs:38-44)
LOG_LEVEL=info                         # error, warn, info, debug, trace (line 129-132)
BASE_URL=http://localhost:8081         # Server base URL (line 67-70)

# === Database Configuration ===
DATABASE_URL="sqlite:./data/users.db"  # SQLite (default) or PostgreSQL (line 134-138)

# === Authentication Configuration ===
# Two-tier key management system (src/crypto/)
PIERRE_MASTER_ENCRYPTION_KEY=$(openssl rand -base64 32)  # Required for production
JWT_EXPIRY_HOURS=24                    # JWT token expiry (line 153-159, default: 24)

# === OAuth Provider Configuration ===
# For fitness provider integration (Strava, Fitbit)
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret
STRAVA_REDIRECT_URI=http://localhost:8081/oauth/callback/strava  # (line 73-76)

FITBIT_CLIENT_ID=your_fitbit_client_id
FITBIT_CLIENT_SECRET=your_fitbit_client_secret
FITBIT_REDIRECT_URI=http://localhost:8081/oauth/callback/fitbit  # (line 79-83)

# === OAuth URLs (customizable) ===
STRAVA_AUTH_URL=https://www.strava.com/oauth/authorize     # (line 87-91)
STRAVA_TOKEN_URL=https://www.strava.com/oauth/token        # (line 94-98)
FITBIT_AUTH_URL=https://www.fitbit.com/oauth2/authorize    # (line 108-112)
FITBIT_TOKEN_URL=https://api.fitbit.com/oauth2/token       # (line 115-119)

# === External Services (Optional) ===
OPENWEATHER_API_KEY=your_api_key       # Weather data integration (optional)
OPENWEATHER_API_BASE=https://api.openweathermap.org/data/2.5  # (line 161-166)

# === Strava API Configuration ===
STRAVA_API_BASE=https://www.strava.com/api/v3  # (line 169-172)
```

### Server Configuration Structure

**Note**: The actual implementation uses a single `http_port` for all protocols (MCP, OAuth 2.0, REST API). See `src/config/environment.rs` for the complete implementation.

```rust
// Example configuration structure (simplified for documentation)
// Actual implementation in src/config/environment.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub environment: Environment,
    pub log_level: LogLevel,
    pub http_port: u16,           // Single unified port for all protocols
    pub base_url: String,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub oauth: OAuthConfig,
    pub rate_limiting: RateLimitingConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        let config = Self {
            environment: Environment::from_str_or_default(
                &env::var("PIERRE_ENVIRONMENT").unwrap_or_default()
            ),
            log_level: LogLevel::from_str_or_default(
                &env::var("PIERRE_LOG_LEVEL").unwrap_or_default()
            ),
            // Single unified port for all protocols (MCP, OAuth 2.0, REST API)
            http_port: env::var("HTTP_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .context("Invalid HTTP port")?,
            base_url: env::var("PIERRE_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            database: DatabaseConfig::from_env()?,
            auth: AuthConfig::from_env()?,
            oauth: OAuthConfig::from_env()?,
            rate_limiting: RateLimitingConfig::from_env()?,
            security: SecurityConfig::from_env()?,
            monitoring: MonitoringConfig::from_env()?,
        };

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        // URL validation
        if !self.base_url.starts_with("http://") && !self.base_url.starts_with("https://") {
            return Err(anyhow!("Base URL must start with http:// or https://"));
        }

        // Production environment validation
        if self.environment.is_production() {
            if !self.base_url.starts_with("https://") {
                return Err(anyhow!("Production environment requires HTTPS"));
            }

            if self.auth.jwt_expiry_hours > 168 {
                return Err(anyhow!("JWT expiry cannot exceed 7 days in production"));
            }
        }

        Ok(())
    }

    pub fn summary(&self) -> String {
        format!(
            "Environment: {}\nHTTP Port: {} (unified for all protocols)\nDatabase: {}\nLog Level: {}",
            self.environment,
            self.http_port,
            self.database.backend_type(),
            self.log_level
        )
    }
}
```

### Database Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: DatabaseUrl,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: u64,
    pub idle_timeout: Option<u64>,
    pub max_lifetime: Option<u64>,
    pub encryption_key_path: PathBuf,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./data/pierre.db".to_string());
        
        Ok(Self {
            url: DatabaseUrl::parse_url(&database_url)?,
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .context("Invalid DATABASE_MAX_CONNECTIONS")?,
            min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("Invalid DATABASE_MIN_CONNECTIONS")?,
            acquire_timeout: env::var("DATABASE_ACQUIRE_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("Invalid DATABASE_ACQUIRE_TIMEOUT")?,
            idle_timeout: env::var("DATABASE_IDLE_TIMEOUT")
                .ok()
                .map(|s| s.parse().context("Invalid DATABASE_IDLE_TIMEOUT"))
                .transpose()?,
            max_lifetime: env::var("DATABASE_MAX_LIFETIME")
                .ok()
                .map(|s| s.parse().context("Invalid DATABASE_MAX_LIFETIME"))
                .transpose()?,
            encryption_key_path: PathBuf::from(
                env::var("DATABASE_ENCRYPTION_KEY_PATH")
                    .unwrap_or_else(|_| "./data/encryption.key".to_string())
            ),
        })
    }
    
    pub fn backend_type(&self) -> &'static str {
        match self.url {
            DatabaseUrl::SQLite { .. } => "SQLite",
            DatabaseUrl::PostgreSQL { .. } => "PostgreSQL",
            DatabaseUrl::Memory => "In-Memory SQLite",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseUrl {
    SQLite { path: PathBuf },
    PostgreSQL { connection_string: String },
    Memory,
}

impl DatabaseUrl {
    pub fn parse_url(s: &str) -> Result<Self> {
        if s.starts_with("sqlite:") {
            let path_str = s.strip_prefix("sqlite:").unwrap_or(s);
            if path_str == ":memory:" {
                Ok(Self::Memory)
            } else {
                Ok(Self::SQLite {
                    path: PathBuf::from(path_str),
                })
            }
        } else if s.starts_with("postgresql://") || s.starts_with("postgres://") {
            Ok(Self::PostgreSQL {
                connection_string: s.to_string(),
            })
        } else {
            // Default to SQLite file
            Ok(Self::SQLite {
                path: PathBuf::from(s),
            })
        }
    }
    
    pub fn to_connection_string(&self) -> String {
        match self {
            Self::SQLite { path } => format!("sqlite:{}", path.display()),
            Self::PostgreSQL { connection_string } => connection_string.clone(),
            Self::Memory => "sqlite::memory:".to_string(),
        }
    }
}
```

### Authentication Configuration

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret_path: PathBuf,
    pub jwt_expiry_hours: u32,
    pub refresh_token_expiry_days: u32,
    pub bcrypt_cost: u32,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u32,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            jwt_secret_path: PathBuf::from(
                env::var("JWT_SECRET_PATH")
                    .unwrap_or_else(|_| "./data/jwt.secret".to_string())
            ),
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .context("Invalid JWT_EXPIRY_HOURS")?,
            refresh_token_expiry_days: env::var("REFRESH_TOKEN_EXPIRY_DAYS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("Invalid REFRESH_TOKEN_EXPIRY_DAYS")?,
            bcrypt_cost: env::var("BCRYPT_COST")
                .unwrap_or_else(|_| "12".to_string())
                .parse()
                .context("Invalid BCRYPT_COST")?,
            max_login_attempts: env::var("MAX_LOGIN_ATTEMPTS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .context("Invalid MAX_LOGIN_ATTEMPTS")?,
            lockout_duration_minutes: env::var("LOCKOUT_DURATION_MINUTES")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .context("Invalid LOCKOUT_DURATION_MINUTES")?,
        })
    }
}
```

## Fitness Configuration

### Fitness Configuration File

```toml
# fitness_config.toml
[sport_types]
"running" = "Running"
"cycling" = "Cycling"  
"swimming" = "Swimming"
"walking" = "Walking"
"hiking" = "Hiking"
"weight_training" = "Weight Training"
"yoga" = "Yoga"
"crossfit" = "CrossFit"

[intelligence]
[intelligence.effort_thresholds]
light_max = 0.6
moderate_max = 0.75
hard_max = 0.9
# > 0.9 = very_high

[intelligence.zone_thresholds]
recovery_max = 0.68
endurance_max = 0.78
tempo_max = 0.87
threshold_max = 0.94
# > 0.94 = vo2max

[intelligence.weather_mapping]
rain_keywords = ["rain", "drizzle", "shower", "precipitation"]
snow_keywords = ["snow", "blizzard", "flurry", "sleet"]
wind_threshold = 15.0  # mph

[intelligence.personal_records]
pace_improvement_threshold = 0.02  # 2% improvement
distance_pr_types = ["5k", "10k", "half_marathon", "marathon"]
time_pr_types = ["30min", "1hour", "2hour"]

[weather_api]
provider = "openweathermap"
enabled = true
cache_duration_hours = 6
request_timeout_seconds = 10
rate_limit_requests_per_minute = 60
```

### Fitness Configuration Implementation

```rust
// src/config/fitness_config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessConfig {
    pub sport_types: HashMap<String, String>,
    pub intelligence: IntelligenceConfig,
    pub weather_api: Option<WeatherApiConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntelligenceConfig {
    pub effort_thresholds: EffortThresholds,
    pub zone_thresholds: ZoneThresholds,
    pub weather_mapping: WeatherMapping,
    pub personal_records: PersonalRecordConfig,
}

impl FitnessConfig {
    pub fn load(path: Option<String>) -> Result<Self> {
        // Try explicit path first
        if let Some(config_path) = path {
            if Path::new(&config_path).exists() {
                return Self::load_from_file(&config_path);
            }
            return Ok(Self::default());
        }
        
        // Try default fitness config file
        if Path::new("fitness_config.toml").exists() {
            return Self::load_from_file("fitness_config.toml");
        }
        
        // Fall back to embedded defaults
        Ok(Self::default())
    }
    
    fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read fitness config file: {}", path))?;
        
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse fitness config file: {}", path))?;
        
        Ok(config)
    }
}

impl Default for FitnessConfig {
    fn default() -> Self {
        Self {
            sport_types: [
                ("running".to_string(), "Running".to_string()),
                ("cycling".to_string(), "Cycling".to_string()),
                ("swimming".to_string(), "Swimming".to_string()),
                ("walking".to_string(), "Walking".to_string()),
                ("hiking".to_string(), "Hiking".to_string()),
                ("weight_training".to_string(), "Weight Training".to_string()),
                ("yoga".to_string(), "Yoga".to_string()),
                ("crossfit".to_string(), "CrossFit".to_string()),
            ].into_iter().collect(),
            intelligence: IntelligenceConfig::default(),
            weather_api: Some(WeatherApiConfig::default()),
        }
    }
}
```

## Docker Configuration

### Docker Compose Configuration

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  pierre-api:
    image: pierre-mcp-server:latest
    ports:
      - "8081:8081"   # HTTP port (includes MCP endpoint at /mcp)
    environment:
      # Server configuration
      PIERRE_ENVIRONMENT: production
      PIERRE_LOG_LEVEL: info
      PIERRE_BASE_URL: https://pierre-api.example.com
      
      # Database configuration
      DATABASE_URL: postgresql://pierre:${DB_PASSWORD}@postgres:5432/pierre
      DATABASE_MAX_CONNECTIONS: 50
      DATABASE_ENCRYPTION_KEY_PATH: /secrets/encryption.key
      
      # Authentication
      JWT_SECRET_PATH: /secrets/jwt.secret
      JWT_EXPIRY_HOURS: 24
      
      # OAuth providers
      STRAVA_CLIENT_ID: ${STRAVA_CLIENT_ID}
      STRAVA_CLIENT_SECRET: ${STRAVA_CLIENT_SECRET}
      STRAVA_REDIRECT_URI: https://pierre-api.example.com/oauth/strava/callback
      
      FITBIT_CLIENT_ID: ${FITBIT_CLIENT_ID}
      FITBIT_CLIENT_SECRET: ${FITBIT_CLIENT_SECRET}
      FITBIT_REDIRECT_URI: https://pierre-api.example.com/oauth/fitbit/callback
      
      # Rate limiting
      RATE_LIMITING_ENABLED: true
      REDIS_URL: redis://redis:6379
      
      # Security
      CORS_ORIGINS: https://pierre.example.com
      SESSION_SECURE: true
      
      # Monitoring
      METRICS_ENABLED: true
      HEALTH_CHECK_ENABLED: true
      
    secrets:
      - encryption_key
      - jwt_secret
      - db_password
    depends_on:
      - postgres
      - redis
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    volumes:
      - ./fitness_config.toml:/app/fitness_config.toml:ro
      - logs:/app/logs
    networks:
      - pierre-network

  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: pierre
      POSTGRES_USER: pierre
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    secrets:
      - db_password
    volumes:
      - postgres_data:/var/lib/postgresql/data
      - ./docker/postgres-init:/docker-entrypoint-initdb.d:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U pierre"]
      interval: 10s
      timeout: 5s
      retries: 5
    networks:
      - pierre-network

  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis_data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3
    networks:
      - pierre-network

secrets:
  encryption_key:
    file: ./secrets/encryption.key
  jwt_secret:
    file: ./secrets/jwt.secret
  db_password:
    file: ./secrets/db_password

volumes:
  postgres_data:
  redis_data:
  logs:

networks:
  pierre-network:
    driver: bridge
```

### Kubernetes Configuration

```yaml
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: pierre-config
  namespace: pierre
data:
  PIERRE_ENVIRONMENT: "production"
  PIERRE_LOG_LEVEL: "info"
  PIERRE_HTTP_PORT: "8081"
  PIERRE_BASE_URL: "https://pierre-api.example.com"
  DATABASE_MAX_CONNECTIONS: "50"
  JWT_EXPIRY_HOURS: "24"
  RATE_LIMITING_ENABLED: "true"
  METRICS_ENABLED: "true"
  CORS_ORIGINS: "https://pierre.example.com"

---
apiVersion: v1
kind: Secret
metadata:
  name: pierre-secrets
  namespace: pierre
type: Opaque
data:
  database-url: <base64-encoded-database-url>
  jwt-secret: <base64-encoded-jwt-secret>
  encryption-key: <base64-encoded-encryption-key>
  strava-client-id: <base64-encoded-strava-client-id>
  strava-client-secret: <base64-encoded-strava-client-secret>

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pierre-api
  namespace: pierre
spec:
  replicas: 3
  selector:
    matchLabels:
      app: pierre-api
  template:
    metadata:
      labels:
        app: pierre-api
    spec:
      containers:
      - name: pierre-api
        image: pierre-mcp-server:latest
        ports:
        - containerPort: 8081
          name: http-mcp
        envFrom:
        - configMapRef:
            name: pierre-config
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: pierre-secrets
              key: database-url
        - name: JWT_SECRET_PATH
          value: "/secrets/jwt.secret"
        - name: DATABASE_ENCRYPTION_KEY_PATH
          value: "/secrets/encryption.key"
        volumeMounts:
        - name: secrets
          mountPath: /secrets
          readOnly: true
        - name: config
          mountPath: /app/fitness_config.toml
          subPath: fitness_config.toml
        livenessProbe:
          httpGet:
            path: /health
            port: 8081
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 8081
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
      volumes:
      - name: secrets
        secret:
          secretName: pierre-secrets
      - name: config
        configMap:
          name: pierre-fitness-config
```

## Configuration Validation

### Runtime Validation

```rust
// src/config/validation.rs
pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate_server_config(config: &ServerConfig) -> Result<Vec<ConfigWarning>> {
        let mut warnings = Vec::new();
        
        // Environment-specific validations
        match config.environment {
            Environment::Production => {
                self.validate_production_config(config, &mut warnings)?;
            }
            Environment::Development => {
                self.validate_development_config(config, &mut warnings)?;
            }
            Environment::Testing => {
                self.validate_testing_config(config, &mut warnings)?;
            }
        }
        
        // General validations
        self.validate_general_config(config, &mut warnings)?;
        
        Ok(warnings)
    }
    
    fn validate_production_config(
        &self,
        config: &ServerConfig,
        warnings: &mut Vec<ConfigWarning>,
    ) -> Result<()> {
        // HTTPS requirement
        if !config.base_url.starts_with("https://") {
            return Err(anyhow!("Production environment requires HTTPS"));
        }
        
        // Strong authentication settings
        if config.auth.bcrypt_cost < 12 {
            warnings.push(ConfigWarning::new(
                "auth.bcrypt_cost",
                "BCrypt cost should be at least 12 in production"
            ));
        }
        
        // Secure JWT settings
        if config.auth.jwt_expiry_hours > 24 {
            warnings.push(ConfigWarning::new(
                "auth.jwt_expiry_hours",
                "JWT expiry should not exceed 24 hours in production"
            ));
        }
        
        // Database settings
        if matches!(config.database.url, DatabaseUrl::SQLite { .. }) {
            warnings.push(ConfigWarning::new(
                "database.url",
                "Consider using PostgreSQL for production deployments"
            ));
        }
        
        Ok(())
    }
    
    fn validate_general_config(
        &self,
        config: &ServerConfig,
        warnings: &mut Vec<ConfigWarning>,
    ) -> Result<()> {
        // Port conflicts
        if config.mcp_port == config.http_port {
            return Err(anyhow!("MCP port and HTTP port cannot be the same"));
        }
        
        // Port ranges
        if config.mcp_port < 1024 || config.http_port < 1024 {
            warnings.push(ConfigWarning::new(
                "ports",
                "Using privileged ports (< 1024) may require root access"
            ));
        }
        
        // Database connection limits
        if config.database.max_connections > 200 {
            warnings.push(ConfigWarning::new(
                "database.max_connections",
                "Very high connection limit may impact performance"
            ));
        }
        
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConfigWarning {
    pub field: String,
    pub message: String,
}

impl ConfigWarning {
    pub fn new(field: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
        }
    }
}
```

## Configuration Management

### Dynamic Configuration Updates

```rust
// src/config/manager.rs
pub struct ConfigManager {
    current_config: Arc<RwLock<ServerConfig>>,
    watchers: Vec<Box<dyn ConfigWatcher>>,
}

impl ConfigManager {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            current_config: Arc::new(RwLock::new(config)),
            watchers: Vec::new(),
        }
    }
    
    pub async fn reload_config(&self) -> Result<()> {
        let new_config = ServerConfig::from_env()?;
        
        // Validate new configuration
        let warnings = ConfigValidator::validate_server_config(&new_config)?;
        for warning in warnings {
            warn!("Config warning for {}: {}", warning.field, warning.message);
        }
        
        // Update configuration
        {
            let mut config = self.current_config.write().await;
            *config = new_config.clone();
        }
        
        // Notify watchers
        for watcher in &self.watchers {
            watcher.on_config_change(&new_config).await?;
        }
        
        info!("Configuration reloaded successfully");
        Ok(())
    }
    
    pub async fn get_config(&self) -> ServerConfig {
        self.current_config.read().await.clone()
    }
    
    pub fn add_watcher(&mut self, watcher: Box<dyn ConfigWatcher>) {
        self.watchers.push(watcher);
    }
}

#[async_trait]
pub trait ConfigWatcher: Send + Sync {
    async fn on_config_change(&self, new_config: &ServerConfig) -> Result<()>;
}
```

### Environment-Specific Configurations

```bash
# Development (.env.development)
PIERRE_ENVIRONMENT=development
PIERRE_LOG_LEVEL=debug
DATABASE_URL=sqlite:./data/dev.db
JWT_EXPIRY_HOURS=72
CORS_ORIGINS=http://localhost:3000,http://localhost:5173
METRICS_ENABLED=false

# Staging (.env.staging)
PIERRE_ENVIRONMENT=production
PIERRE_LOG_LEVEL=info
PIERRE_BASE_URL=https://staging-pierre-api.example.com
DATABASE_URL=postgresql://pierre:${DB_PASSWORD}@staging-db.example.com:5432/pierre_staging
JWT_EXPIRY_HOURS=24
CORS_ORIGINS=https://staging-pierre.example.com
METRICS_ENABLED=true

# Production (.env.production)
PIERRE_ENVIRONMENT=production
PIERRE_LOG_LEVEL=warn
PIERRE_BASE_URL=https://pierre-api.example.com
DATABASE_URL=postgresql://pierre:${DB_PASSWORD}@prod-db.example.com:5432/pierre
JWT_EXPIRY_HOURS=12
CORS_ORIGINS=https://pierre.example.com
METRICS_ENABLED=true
AUDIT_LOGGING=true
```

## Configuration Testing

### Configuration Tests

```rust
// tests/config_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_default_config() {
        // Clear environment variables
        env::remove_var("PIERRE_ENVIRONMENT");
        env::remove_var("DATABASE_URL");
        
        let config = ServerConfig::from_env().unwrap();
        
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.mcp_port, 8081);  // Unified port architecture
        assert_eq!(config.http_port, 8081);
        assert!(matches!(config.database.url, DatabaseUrl::SQLite { .. }));
    }
    
    #[test]
    fn test_production_config_validation() {
        env::set_var("PIERRE_ENVIRONMENT", "production");
        env::set_var("PIERRE_BASE_URL", "http://example.com"); // HTTP in prod
        
        let result = ServerConfig::from_env();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));
    }
    
    #[test]
    fn test_database_url_parsing() {
        let sqlite_url = DatabaseUrl::parse_url("sqlite:./test.db").unwrap();
        assert!(matches!(sqlite_url, DatabaseUrl::SQLite { .. }));
        
        let pg_url = DatabaseUrl::parse_url("postgresql://user:pass@host:5432/db").unwrap();
        assert!(matches!(pg_url, DatabaseUrl::PostgreSQL { .. }));
        
        let memory_url = DatabaseUrl::parse_url("sqlite::memory:").unwrap();
        assert!(matches!(memory_url, DatabaseUrl::Memory));
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = ServerConfig::default();
        config.environment = Environment::Production;
        config.base_url = "https://example.com".to_string();
        
        let warnings = ConfigValidator::validate_server_config(&config).unwrap();
        assert!(!warnings.is_empty()); // Should have warnings about SQLite in prod
    }
}
```

This comprehensive configuration system provides flexible, validated, and environment-aware configuration management for Pierre MCP Server deployments.