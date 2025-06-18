#!/usr/bin/env python3
"""
Common Data Processing Utilities
Shared data analysis and processing functions for fitness data
"""

import json
import statistics
from datetime import datetime, timedelta
from typing import List, Dict, Optional, Tuple

class FitnessDataProcessor:
    """Advanced fitness data processing and analysis"""
    
    @staticmethod
    def analyze_activity_distribution(activities: List[Dict]) -> Dict:
        """Analyze sport and intensity distribution"""
        if not activities:
            return {}
        
        sport_counts = {}
        high_intensity = []
        moderate_intensity = []
        low_intensity = []
        
        for activity in activities:
            # Sport distribution
            sport = activity.get('sport_type', 'unknown')
            if isinstance(sport, dict):
                sport = 'complex_sport_type'
            elif not isinstance(sport, str):
                sport = str(sport)
            
            sport_counts[sport] = sport_counts.get(sport, 0) + 1
            
            # Intensity classification
            duration_s = activity.get('moving_time_seconds', 0) or activity.get('duration_seconds', 0)
            
            if sport.lower() == 'run':
                high_intensity.append(activity)
            elif sport.lower() == 'ride' and duration_s > 1800:  # 30+ min cycling
                high_intensity.append(activity)
            elif sport.lower() in ['hike', 'walk'] and duration_s > 1200:  # 20+ min
                moderate_intensity.append(activity)
            elif sport.lower() == 'ride':  # Shorter cycling
                moderate_intensity.append(activity)
            else:
                low_intensity.append(activity)
        
        return {
            'sport_distribution': sport_counts,
            'intensity_distribution': {
                'high': len(high_intensity),
                'moderate': len(moderate_intensity), 
                'low': len(low_intensity)
            },
            'high_intensity_activities': high_intensity,
            'moderate_intensity_activities': moderate_intensity,
            'low_intensity_activities': low_intensity
        }
    
    @staticmethod
    def calculate_training_metrics(activities: List[Dict]) -> Dict:
        """Calculate comprehensive training metrics"""
        if not activities:
            return {}
        
        # Extract metrics
        distances = []
        durations = []
        elevations = []
        dates = []
        
        total_distance = 0
        total_duration = 0
        total_elevation = 0
        
        for activity in activities:
            distance_m = activity.get('distance_meters', 0)
            duration_s = activity.get('moving_time_seconds', 0) or activity.get('duration_seconds', 0)
            elevation_m = activity.get('elevation_gain', 0)
            date_str = activity.get('start_date', '')
            
            if distance_m:
                distance_km = distance_m / 1000
                distances.append(distance_km)
                total_distance += distance_km
            
            if duration_s:
                duration_min = duration_s / 60
                durations.append(duration_min)
                total_duration += duration_min
            
            if elevation_m:
                elevations.append(elevation_m)
                total_elevation += elevation_m
            
            if date_str:
                try:
                    dates.append(datetime.fromisoformat(date_str.replace('Z', '+00:00')))
                except:
                    pass
        
        # Calculate timespan and frequency
        if dates:
            dates.sort()
            timespan_days = (dates[-1] - dates[0]).days + 1
            activities_per_week = len(activities) / (timespan_days / 7) if timespan_days > 0 else 0
        else:
            timespan_days = 0
            activities_per_week = 0
        
        return {
            'totals': {
                'activities': len(activities),
                'distance_km': total_distance,
                'duration_hours': total_duration / 60,
                'elevation_m': total_elevation
            },
            'averages': {
                'distance_km': statistics.mean(distances) if distances else 0,
                'duration_min': statistics.mean(durations) if durations else 0,
                'elevation_m': statistics.mean(elevations) if elevations else 0
            },
            'frequency': {
                'activities_per_week': activities_per_week,
                'timespan_days': timespan_days
            },
            'ranges': {
                'distance_range': (min(distances), max(distances)) if distances else (0, 0),
                'duration_range': (min(durations), max(durations)) if durations else (0, 0)
            }
        }
    
    @staticmethod
    def calculate_fitness_score(activities: List[Dict]) -> Dict:
        """Calculate comprehensive fitness score"""
        if not activities:
            return {'total_score': 0, 'components': {}, 'insights': []}
        
        distribution = FitnessDataProcessor.analyze_activity_distribution(activities)
        metrics = FitnessDataProcessor.calculate_training_metrics(activities)
        
        # 1. Frequency Score (25 points)
        activities_per_week = metrics['frequency']['activities_per_week']
        if activities_per_week >= 7:
            frequency_score = 25
        elif activities_per_week >= 5:
            frequency_score = 22
        elif activities_per_week >= 3:
            frequency_score = 18
        else:
            frequency_score = max(0, int(activities_per_week * 6))
        
        # 2. Quality/Intensity Score (25 points)
        intensity_dist = distribution['intensity_distribution']
        total_activities = sum(intensity_dist.values())
        
        if total_activities > 0:
            high_ratio = intensity_dist['high'] / total_activities
            moderate_ratio = intensity_dist['moderate'] / total_activities
            
            intensity_score = int(
                (high_ratio * 25) + 
                (moderate_ratio * 20) + 
                ((1 - high_ratio - moderate_ratio) * 10)
            )
        else:
            intensity_score = 0
        
        intensity_score = min(25, intensity_score)
        
        # 3. Consistency Score (25 points)
        timespan_days = metrics['frequency']['timespan_days']
        if timespan_days >= 60 and activities_per_week >= 7:
            consistency_score = 25
        elif timespan_days >= 30 and activities_per_week >= 5:
            consistency_score = 22
        elif timespan_days >= 14 and activities_per_week >= 3:
            consistency_score = 18
        else:
            consistency_score = 15
        
        # 4. Variety Score (25 points)
        sport_count = len(distribution['sport_distribution'])
        variety_base = min(sport_count * 3, 15)
        
        # Bonus for quality variety
        sports = set(distribution['sport_distribution'].keys())
        variety_bonus = 0
        
        has_running = any('run' in s.lower() for s in sports)
        has_cycling = any('ride' in s.lower() for s in sports)
        has_other_cardio = any(s.lower() in ['hike', 'kayaking', 'swim'] for s in sports)
        
        if has_running and has_cycling:
            variety_bonus += 8
        if has_other_cardio:
            variety_bonus += 2
        
        variety_score = min(25, variety_base + variety_bonus)
        
        total_score = frequency_score + intensity_score + consistency_score + variety_score
        
        # Generate insights
        insights = []
        
        if total_score >= 90:
            insights.append("ELITE fitness level - exceptional commitment and performance")
        elif total_score >= 80:
            insights.append("EXCELLENT fitness level - serious athlete dedication")
        elif total_score >= 70:
            insights.append("GOOD fitness level - committed recreational athlete")
        else:
            insights.append("DEVELOPING fitness - building consistent training base")
        
        if frequency_score >= 23:
            insights.append("Outstanding training frequency - daily activity pattern")
        
        if intensity_score >= 20:
            insights.append("Excellent intensity balance - quality training distribution")
        
        if consistency_score >= 23:
            insights.append("Exceptional consistency - sustained long-term commitment")
        
        if variety_score >= 20:
            insights.append("Great sport variety - well-rounded athletic development")
        
        return {
            'total_score': total_score,
            'components': {
                'frequency': frequency_score,
                'intensity': intensity_score,
                'consistency': consistency_score,
                'variety': variety_score
            },
            'insights': insights,
            'metrics': metrics,
            'distribution': distribution
        }
    
    @staticmethod
    def filter_by_sport(activities: List[Dict], sport_type: str) -> List[Dict]:
        """Filter activities by sport type"""
        return [
            activity for activity in activities
            if isinstance(activity.get('sport_type'), str) and 
               activity.get('sport_type').lower() == sport_type.lower()
        ]
    
    @staticmethod
    def analyze_running_performance(activities: List[Dict]) -> Dict:
        """Specialized running performance analysis"""
        running_activities = FitnessDataProcessor.filter_by_sport(activities, 'run')
        
        if not running_activities:
            return {'error': 'No running activities found'}
        
        distances = []
        paces = []
        elevations = []
        
        for run in running_activities:
            distance_m = run.get('distance_meters', 0)
            duration_s = run.get('moving_time_seconds', 0) or run.get('duration_seconds', 0)
            elevation_m = run.get('elevation_gain', 0)
            
            if distance_m and distance_m > 0:
                distance_km = distance_m / 1000
                distances.append(distance_km)
                
                if duration_s and duration_s > 0:
                    pace_min_per_km = (duration_s / 60) / distance_km
                    paces.append(pace_min_per_km)
            
            if elevation_m:
                elevations.append(elevation_m)
        
        # Categorize runs by distance
        short_runs = [d for d in distances if d <= 5]
        medium_runs = [d for d in distances if 5 < d <= 10]
        long_runs = [d for d in distances if d > 10]
        
        return {
            'total_runs': len(running_activities),
            'total_distance': sum(distances),
            'average_distance': statistics.mean(distances) if distances else 0,
            'average_pace': statistics.mean(paces) if paces else 0,
            'average_elevation': statistics.mean(elevations) if elevations else 0,
            'distance_distribution': {
                'short_runs': len(short_runs),
                'medium_runs': len(medium_runs),
                'long_runs': len(long_runs)
            }
        }

class DataAnonymizer:
    """Data anonymization utilities for privacy protection"""
    
    @staticmethod
    def anonymize_activity(activity: Dict) -> Dict:
        """Remove or anonymize personal data from activity"""
        anonymized = activity.copy()
        
        # Remove personal identifiers
        if 'name' in anonymized:
            anonymized['name'] = f"Activity_{anonymized.get('id', 'unknown')}"
        
        # Remove precise GPS coordinates
        if 'start_latitude' in anonymized:
            anonymized.pop('start_latitude')
        if 'start_longitude' in anonymized:
            anonymized.pop('start_longitude')
        
        # Remove location details
        for field in ['city', 'country', 'state', 'address']:
            if field in anonymized:
                anonymized.pop(field)
        
        # Keep only essential fitness data
        essential_fields = {
            'id', 'sport_type', 'distance_meters', 'duration_seconds', 
            'moving_time_seconds', 'elevation_gain', 'start_date',
            'average_heart_rate', 'max_heart_rate', 'calories',
            'provider', 'is_real_data'
        }
        
        # Remove any fields not in essential list
        anonymized = {k: v for k, v in anonymized.items() if k in essential_fields}
        
        return anonymized
    
    @staticmethod
    def anonymize_activity_list(activities: List[Dict]) -> List[Dict]:
        """Anonymize a list of activities"""
        return [DataAnonymizer.anonymize_activity(activity) for activity in activities]

class DataValidator:
    """Data quality validation utilities"""
    
    @staticmethod
    def validate_activity_data(activities: List[Dict]) -> Dict:
        """Validate activity data quality and completeness"""
        if not activities:
            return {
                'valid': False,
                'issues': ['No activities provided'],
                'quality_score': 0
            }
        
        issues = []
        
        # Check data completeness
        has_distance = sum(1 for a in activities if a.get('distance_meters', 0) > 0)
        has_duration = sum(1 for a in activities if 
                          a.get('moving_time_seconds', 0) > 0 or a.get('duration_seconds', 0) > 0)
        has_sport_type = sum(1 for a in activities if a.get('sport_type'))
        has_dates = sum(1 for a in activities if a.get('start_date'))
        
        total = len(activities)
        distance_pct = has_distance / total
        duration_pct = has_duration / total
        sport_pct = has_sport_type / total
        date_pct = has_dates / total
        
        if distance_pct < 0.8:
            issues.append(f"Low distance completeness: {distance_pct:.1%}")
        if duration_pct < 0.9:
            issues.append(f"Low duration completeness: {duration_pct:.1%}")
        if sport_pct < 0.95:
            issues.append(f"Low sport type completeness: {sport_pct:.1%}")
        if date_pct < 0.95:
            issues.append(f"Low date completeness: {date_pct:.1%}")
        
        quality_score = (distance_pct + duration_pct + sport_pct + date_pct) / 4 * 100
        
        return {
            'valid': quality_score >= 70,
            'quality_score': quality_score,
            'completeness': {
                'distance': distance_pct,
                'duration': duration_pct,
                'sport_type': sport_pct,
                'dates': date_pct
            },
            'issues': issues
        }

def main():
    """Test data processing utilities"""
    print("📊 Data Processing Utilities Test")
    print("=" * 40)
    
    # Load sample data if available
    try:
        with open('enterprise_strava_dataset.json', 'r') as f:
            data = json.load(f)
            if isinstance(data, dict) and 'activities' in data:
                activities = data['activities']
            else:
                activities = data
        
        print(f"✅ Loaded {len(activities)} activities for testing")
        
        # Test fitness score calculation
        fitness_results = FitnessDataProcessor.calculate_fitness_score(activities)
        print(f"\n🏆 Fitness Score: {fitness_results['total_score']}/100")
        
        components = fitness_results['components']
        for component, score in components.items():
            print(f"   • {component.title()}: {score}/25")
        
        # Test data validation
        validation = DataValidator.validate_activity_data(activities)
        print(f"\n✅ Data Quality: {validation['quality_score']:.1f}/100")
        
        if validation['issues']:
            print("⚠️ Issues found:")
            for issue in validation['issues']:
                print(f"   • {issue}")
        
        # Test running analysis
        running_analysis = FitnessDataProcessor.analyze_running_performance(activities)
        if 'error' not in running_analysis:
            print(f"\n🏃 Running Analysis:")
            print(f"   • Total runs: {running_analysis['total_runs']}")
            print(f"   • Average distance: {running_analysis['average_distance']:.1f}km")
            print(f"   • Average pace: {running_analysis['average_pace']:.1f} min/km")
        
    except FileNotFoundError:
        print("⚠️ No sample data found - run data collection first")

if __name__ == "__main__":
    main()