// ABOUTME: Mobility repository implementation for stretching exercises and yoga poses
// ABOUTME: Provides read-only access to seeded mobility data with filtering and search
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::MobilityRepository;
use crate::database::mobility::{
    ActivityMuscleMapping, ListStretchingFilter, ListYogaFilter, MobilityManager,
    StretchingExercise, YogaPose,
};
use crate::database::DatabaseError;
use crate::database_plugins::factory::Database;
use async_trait::async_trait;

/// SQLite/PostgreSQL implementation of `MobilityRepository`
pub struct MobilityRepositoryImpl {
    db: Database,
}

impl MobilityRepositoryImpl {
    /// Create a new `MobilityRepository` with the given database connection
    #[must_use]
    pub const fn new(db: Database) -> Self {
        Self { db }
    }

    fn get_manager(&self) -> Option<MobilityManager> {
        self.db
            .sqlite_pool()
            .map(|pool| MobilityManager::new(pool.clone()))
    }
}

#[async_trait]
impl MobilityRepository for MobilityRepositoryImpl {
    async fn get_stretching_exercise(
        &self,
        id: &str,
    ) -> Result<Option<StretchingExercise>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .get_stretching_exercise(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_stretching_exercises(
        &self,
        filter: &ListStretchingFilter,
    ) -> Result<Vec<StretchingExercise>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .list_stretching_exercises(filter)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn search_stretching_exercises(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<StretchingExercise>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .search_stretching_exercises(query, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_stretches_for_activity(
        &self,
        activity_type: &str,
        limit: Option<u32>,
    ) -> Result<Vec<StretchingExercise>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .get_stretches_for_activity(activity_type, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_yoga_pose(&self, id: &str) -> Result<Option<YogaPose>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .get_yoga_pose(id)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_yoga_poses(&self, filter: &ListYogaFilter) -> Result<Vec<YogaPose>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .list_yoga_poses(filter)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn search_yoga_poses(
        &self,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<YogaPose>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .search_yoga_poses(query, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_poses_for_recovery(
        &self,
        recovery_context: &str,
        limit: Option<u32>,
    ) -> Result<Vec<YogaPose>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .get_poses_for_recovery(recovery_context, limit)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn get_activity_muscle_mapping(
        &self,
        activity_type: &str,
    ) -> Result<Option<ActivityMuscleMapping>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .get_activity_muscle_mapping(activity_type)
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }

    async fn list_activity_muscle_mappings(
        &self,
    ) -> Result<Vec<ActivityMuscleMapping>, DatabaseError> {
        let manager = self.get_manager().ok_or_else(|| DatabaseError::QueryError {
            context: "Mobility operations require SQLite backend".to_string(),
        })?;

        manager
            .list_activity_muscle_mappings()
            .await
            .map_err(|e| DatabaseError::QueryError {
                context: e.to_string(),
            })
    }
}
