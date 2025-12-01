// ABOUTME: Autonomous scheduling and reporting system for fitness analysis
// ABOUTME: Demonstrates continuous agent operation with configurable intervals
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use crate::a2a_client::A2AClient;
use crate::analyzer::{AnalysisResults, FitnessAnalyzer};
use crate::config::AgentConfig;

/// Report metadata
#[derive(Debug, Serialize, Deserialize)]
struct AnalysisReport {
    generated_at: DateTime<Utc>,
    agent_version: String,
    config_snapshot: ConfigSnapshot,
    analysis_results: AnalysisResults,
    execution_stats: ExecutionStats,
}

/// Configuration snapshot for reporting
#[derive(Debug, Serialize, Deserialize)]
struct ConfigSnapshot {
    server_url: String,
    analysis_interval_hours: u64,
    max_activities_per_analysis: u32,
    development_mode: bool,
}

/// Execution statistics
#[derive(Debug, Serialize, Deserialize)]
struct ExecutionStats {
    analysis_duration_ms: u64,
    a2a_requests_made: u32,
    errors_encountered: u32,
    memory_usage_estimate_kb: u64,
}

/// Analysis scheduler for autonomous operation
pub struct AnalysisScheduler {
    config: AgentConfig,
    analyzer: FitnessAnalyzer,
    execution_count: u64,
}

impl AnalysisScheduler {
    /// Create a new analysis scheduler
    pub async fn new(config: AgentConfig) -> Result<Self> {
        info!("🚀 Initializing Analysis Scheduler");

        // Create A2A client
        let a2a_client = A2AClient::new(
            config.server_url.clone(),
            config.client_id.clone(),
            config.client_secret.clone(),
        );

        // Create fitness analyzer
        let analyzer = FitnessAnalyzer::new(a2a_client);

        // Ensure report output directory exists
        if config.generate_reports {
            fs::create_dir_all(&config.report_output_dir)
                .context("Failed to create report output directory")?;
            info!("📁 Report directory: {}", config.report_output_dir);
        }

        Ok(Self {
            config,
            analyzer,
            execution_count: 0,
        })
    }

    /// Run the analysis scheduler (main loop)
    pub async fn run(&mut self) -> Result<()> {
        info!("▶️ Starting autonomous analysis scheduler");
        info!("⏰ Analysis interval: {:?}", self.config.dev_analysis_interval());

        // Initial authentication test
        self.test_a2a_connection().await?;

        if self.config.development_mode {
            info!("🚧 Development mode: Running single analysis then exiting");
            self.perform_analysis_cycle().await?;
            return Ok(());
        }

        // Production mode: continuous operation
        let mut analysis_timer = interval(self.config.dev_analysis_interval());

        loop {
            analysis_timer.tick().await;
            
            match self.perform_analysis_cycle().await {
                Ok(()) => {
                    info!("✅ Analysis cycle {} completed successfully", self.execution_count);
                }
                Err(e) => {
                    error!("❌ Analysis cycle {} failed: {}", self.execution_count, e);
                    
                    // In production, continue running despite errors
                    // In development, we might want to exit on errors
                    if self.config.development_mode {
                        return Err(e);
                    } else {
                        warn!("🔄 Continuing despite error in production mode");
                    }
                }
            }
        }
    }

    /// Test A2A connection during initialization
    async fn test_a2a_connection(&mut self) -> Result<()> {
        info!("🔌 Testing A2A connection...");
        
        // Try to authenticate
        self.analyzer.client.authenticate().await
            .context("A2A authentication test failed")?;
        
        info!("✅ A2A connection test successful");
        Ok(())
    }

    /// Perform a single analysis cycle
    async fn perform_analysis_cycle(&mut self) -> Result<()> {
        self.execution_count += 1;
        let cycle_start = std::time::Instant::now();

        info!("🔬 Starting analysis cycle #{}", self.execution_count);

        // Perform fitness analysis via A2A
        let analysis_start = std::time::Instant::now();
        let analysis_results = self.analyzer
            .analyze("strava", self.config.max_activities_per_analysis)
            .await
            .context("Fitness analysis failed")?;
        let analysis_duration = analysis_start.elapsed();

        // Log key results
        self.log_analysis_summary(&analysis_results);

        // Generate report if configured
        if self.config.generate_reports {
            self.generate_report(&analysis_results, analysis_duration).await?;
        }

        // Log cycle completion
        let cycle_duration = cycle_start.elapsed();
        info!("⏱️ Analysis cycle completed in {:.2}s", cycle_duration.as_secs_f64());

        // In development mode, provide detailed output
        if self.config.development_mode {
            self.display_detailed_results(&analysis_results);
        }

        Ok(())
    }

    /// Log summary of analysis results
    fn log_analysis_summary(&self, results: &AnalysisResults) {
        info!("📊 Analysis Summary:");
        info!("  • Activities analyzed: {}", results.activities_analyzed);
        info!("  • Patterns detected: {}", results.patterns.len());
        info!("  • Recommendations: {}", results.recommendations.len());
        info!("  • Risk indicators: {}", results.risk_indicators.len());
        info!("  • Performance trend: {}", results.performance_trends.overall_trend);

        // Log high-priority risks
        for risk in &results.risk_indicators {
            if risk.severity == "high" {
                warn!("⚠️ HIGH RISK: {} ({}% probability)", risk.description, (risk.probability * 100.0) as u8);
            }
        }

        // Log high-priority recommendations
        for rec in &results.recommendations {
            if rec.priority == "high" {
                info!("🎯 PRIORITY: {}", rec.title);
            }
        }
    }

    /// Display detailed results (development mode)
    fn display_detailed_results(&self, results: &AnalysisResults) {
        println!("\n🔍 DETAILED ANALYSIS RESULTS");
        println!("{}", "=".repeat(50));

        // Patterns
        if !results.patterns.is_empty() {
            println!("\n📈 DETECTED PATTERNS:");
            for (i, pattern) in results.patterns.iter().enumerate() {
                println!("{}. {} (confidence: {:.1}%)",
                    i + 1, pattern.description, pattern.confidence * 100.0);
            }
        }

        // Recommendations
        if !results.recommendations.is_empty() {
            println!("\n💡 RECOMMENDATIONS:");
            for (i, rec) in results.recommendations.iter().enumerate() {
                println!("{}. [{}] {}: {}",
                    i + 1, rec.priority.to_uppercase(), rec.title, rec.description);
            }
        }

        // Risks
        if !results.risk_indicators.is_empty() {
            println!("\n⚠️ RISK INDICATORS:");
            for (i, risk) in results.risk_indicators.iter().enumerate() {
                println!("{}. [{}] {} ({}% probability)",
                    i + 1, risk.severity.to_uppercase(), risk.description, 
                    (risk.probability * 100.0) as u8);
            }
        }

        // Performance trends
        println!("\n📊 PERFORMANCE TRENDS:");
        println!("  Overall: {}", results.performance_trends.overall_trend);
        if let Some(pace) = results.performance_trends.pace_trend {
            println!("  Pace trend: {:.3} sec/m per activity", pace);
        }
        if let Some(distance) = results.performance_trends.distance_trend {
            println!("  Distance trend: {:.1} meters per activity", distance);
        }
        if let Some(frequency) = results.performance_trends.frequency_trend {
            println!("  Frequency trend: {:.1} activities/week change", frequency);
        }

        println!("{}", "=".repeat(50));
    }

    /// Generate analysis report
    async fn generate_report(
        &self,
        analysis_results: &AnalysisResults,
        analysis_duration: Duration,
    ) -> Result<()> {
        let report_timestamp = Utc::now();
        let report_filename = format!(
            "fitness_analysis_report_{}.json",
            report_timestamp.format("%Y%m%d_%H%M%S")
        );
        let report_path = Path::new(&self.config.report_output_dir).join(&report_filename);

        // Estimate memory usage (rough approximation)
        let memory_estimate = self.estimate_memory_usage(analysis_results);

        let execution_stats = ExecutionStats {
            analysis_duration_ms: analysis_duration.as_millis() as u64,
            a2a_requests_made: self.estimate_a2a_requests(analysis_results),
            errors_encountered: 0, // Could be tracked more precisely
            memory_usage_estimate_kb: memory_estimate,
        };

        let config_snapshot = ConfigSnapshot {
            server_url: self.config.server_url.clone(),
            analysis_interval_hours: self.config.analysis_interval_hours,
            max_activities_per_analysis: self.config.max_activities_per_analysis,
            development_mode: self.config.development_mode,
        };

        let report = AnalysisReport {
            generated_at: report_timestamp,
            agent_version: "1.0.0".to_string(),
            config_snapshot,
            analysis_results: analysis_results.clone(),
            execution_stats,
        };

        // Write report to file
        let report_json = serde_json::to_string_pretty(&report)
            .context("Failed to serialize analysis report")?;

        fs::write(&report_path, report_json)
            .context("Failed to write analysis report")?;

        info!("📄 Analysis report saved: {}", report_filename);

        // Clean up old reports (keep last 10)
        self.cleanup_old_reports().await?;

        Ok(())
    }

    /// Cleanup old report files
    async fn cleanup_old_reports(&self) -> Result<()> {
        let report_dir = Path::new(&self.config.report_output_dir);
        
        let mut report_files = Vec::new();
        if let Ok(entries) = fs::read_dir(report_dir) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.starts_with("fitness_analysis_report_") && filename.ends_with(".json") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                report_files.push((entry.path(), modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modification time (newest first)
        report_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Remove old files (keep the 10 most recent)
        for (path, _) in report_files.iter().skip(10) {
            if let Err(e) = fs::remove_file(path) {
                warn!("Failed to remove old report {:?}: {}", path, e);
            } else {
                info!("🗑️ Cleaned up old report: {:?}", path.file_name());
            }
        }

        Ok(())
    }

    /// Estimate memory usage (rough approximation)
    fn estimate_memory_usage(&self, results: &AnalysisResults) -> u64 {
        // Very rough estimate based on JSON serialization size
        if let Ok(json) = serde_json::to_string(results) {
            (json.len() * 4) as u64 // Assume 4x overhead for in-memory representation
        } else {
            1024 // Default 1KB estimate
        }
    }

    /// Estimate number of A2A requests made
    fn estimate_a2a_requests(&self, results: &AnalysisResults) -> u32 {
        let mut requests = 1; // At least one for get_activities
        
        // Additional requests for recommendations and metrics
        if !results.recommendations.is_empty() {
            requests += 1; // generate_recommendations
        }
        
        // Could be more sophisticated based on actual tracking
        requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{PerformanceTrends, AnalysisResults};

    fn create_test_results() -> AnalysisResults {
        AnalysisResults {
            timestamp: Utc::now(),
            activities_analyzed: 10,
            patterns: vec![],
            recommendations: vec![],
            risk_indicators: vec![],
            performance_trends: PerformanceTrends {
                overall_trend: "stable".to_string(),
                pace_trend: None,
                distance_trend: None,
                frequency_trend: None,
                heart_rate_trend: None,
            },
        }
    }

    #[test]
    fn test_memory_usage_estimation() {
        let config = AgentConfig::default();
        let a2a_client = A2AClient::new(
            "http://test".to_string(),
            "test".to_string(),
            "test".to_string(),
        );
        let analyzer = FitnessAnalyzer::new(a2a_client);
        let scheduler = AnalysisScheduler {
            config,
            analyzer,
            execution_count: 0,
        };

        let results = create_test_results();
        let memory_estimate = scheduler.estimate_memory_usage(&results);
        
        assert!(memory_estimate > 0);
        assert!(memory_estimate < 100_000); // Should be reasonable
    }

    #[test]
    fn test_a2a_request_estimation() {
        let config = AgentConfig::default();
        let a2a_client = A2AClient::new(
            "http://test".to_string(),
            "test".to_string(),
            "test".to_string(),
        );
        let analyzer = FitnessAnalyzer::new(a2a_client);
        let scheduler = AnalysisScheduler {
            config,
            analyzer,
            execution_count: 0,
        };

        let results = create_test_results();
        let request_count = scheduler.estimate_a2a_requests(&results);
        
        assert!(request_count >= 1); // At least one request
    }

    #[test]
    fn test_config_snapshot_creation() {
        let config = AgentConfig {
            server_url: "http://test:8081".to_string(),
            analysis_interval_hours: 24,
            max_activities_per_analysis: 100,
            development_mode: true,
            ..Default::default()
        };

        let snapshot = ConfigSnapshot {
            server_url: config.server_url.clone(),
            analysis_interval_hours: config.analysis_interval_hours,
            max_activities_per_analysis: config.max_activities_per_analysis,
            development_mode: config.development_mode,
        };

        assert_eq!(snapshot.server_url, "http://test:8081");
        assert_eq!(snapshot.analysis_interval_hours, 24);
        assert!(snapshot.development_mode);
    }
}