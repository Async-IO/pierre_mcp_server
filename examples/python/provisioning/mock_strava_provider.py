#!/usr/bin/env python3
"""
Mock Strava Provider for Provisioning Examples
Generates realistic sample fitness data for testing provisioning workflows
"""

import json
import math
import random
from datetime import datetime, timedelta
from typing import List, Dict, Any
from dataclasses import dataclass, asdict

@dataclass
class MockActivity:
    """Mock Strava activity with realistic data"""
    id: int
    name: str
    type: str
    distance: float  # meters
    moving_time: int  # seconds
    elapsed_time: int  # seconds
    start_date: str
    start_latlng: List[float]
    end_latlng: List[float]
    average_speed: float  # m/s
    max_speed: float  # m/s
    elevation_gain: float  # meters
    kudos_count: int
    comment_count: int
    athlete_count: int
    total_elevation_gain: float
    suffer_score: int
    average_heartrate: float
    max_heartrate: float
    calories: float

class MockStravaProvider:
    """Mock Strava provider generating realistic sample data"""
    
    def __init__(self, athlete_name: str = "Demo Athlete"):
        self.athlete_name = athlete_name
        self.base_date = datetime.now() - timedelta(days=365)
        
        # Activity types with realistic distributions
        self.activity_types = {
            'Run': {'weight': 60, 'distance_range': (3000, 25000), 'speed_range': (3.0, 6.0)},
            'Ride': {'weight': 25, 'distance_range': (10000, 120000), 'speed_range': (8.0, 15.0)},
            'Swim': {'weight': 10, 'distance_range': (500, 4000), 'speed_range': (1.2, 2.0)},
            'Hike': {'weight': 5, 'distance_range': (2000, 15000), 'speed_range': (1.0, 2.5)}
        }
        
        # Location clusters (lat, lng) for realistic GPS data
        self.locations = [
            [37.7749, -122.4194],  # San Francisco
            [37.7849, -122.4094],  # SF North
            [37.7649, -122.4294],  # SF South
            [37.8049, -122.4094],  # SF Bay Area
        ]
        
        # Training progression - athlete gets fitter over time
        self.fitness_progression = self._generate_fitness_curve()
    
    def _generate_fitness_curve(self) -> List[float]:
        """Generate realistic fitness progression over a year"""
        # Base fitness that improves over time with some variation
        days = 365
        base_fitness = []
        current_fitness = 0.7  # Start at 70% fitness
        
        for day in range(days):
            # Seasonal variation (better in summer, slower in winter)
            seasonal_factor = 0.1 * math.sin(2 * math.pi * day / 365) + 1.0
            
            # Weekly training cycle (rest days vs. training days)
            weekly_factor = 0.9 if day % 7 in [0, 6] else 1.0  # Rest on weekends
            
            # Gradual improvement with some randomness
            improvement = 0.0003 + random.uniform(-0.0002, 0.0002)
            current_fitness = min(1.0, current_fitness + improvement)
            
            daily_fitness = current_fitness * seasonal_factor * weekly_factor
            base_fitness.append(max(0.3, daily_fitness))  # Never go below 30%
        
        return base_fitness
    
    def _get_activity_type(self) -> str:
        """Randomly select activity type based on realistic distribution"""
        weights = [data['weight'] for data in self.activity_types.values()]
        return random.choices(list(self.activity_types.keys()), weights=weights)[0]
    
    def _generate_gps_points(self, start_lat: float, start_lng: float, 
                           distance: float) -> List[List[float]]:
        """Generate realistic GPS route"""
        # Simple linear route with some variation
        num_points = min(100, max(10, int(distance / 100)))  # 1 point per 100m
        points = []
        
        # Random direction
        bearing = random.uniform(0, 360)
        
        for i in range(num_points):
            # Distance per point in degrees (very rough approximation)
            progress = i / num_points
            point_distance = (distance * progress) / 111000  # Convert meters to degrees
            
            # Add some randomness to create a realistic route
            noise_lat = random.uniform(-0.0005, 0.0005)
            noise_lng = random.uniform(-0.0005, 0.0005)
            
            lat = start_lat + point_distance * math.cos(math.radians(bearing)) + noise_lat
            lng = start_lng + point_distance * math.sin(math.radians(bearing)) + noise_lng
            
            points.append([lat, lng])
        
        return points
    
    def generate_activity(self, activity_id: int, days_ago: int) -> MockActivity:
        """Generate a single realistic activity"""
        activity_type = self._get_activity_type()
        type_config = self.activity_types[activity_type]
        
        # Get fitness level for this day
        fitness_day = max(0, len(self.fitness_progression) - days_ago - 1)
        fitness_level = self.fitness_progression[fitness_day] if fitness_day < len(self.fitness_progression) else 0.7
        
        # Generate activity parameters based on type and fitness
        distance = random.uniform(*type_config['distance_range']) * fitness_level
        speed = random.uniform(*type_config['speed_range']) * (0.8 + 0.4 * fitness_level)
        
        moving_time = int(distance / speed)
        elapsed_time = moving_time + random.randint(30, 300)  # Stops/breaks
        
        # Location
        start_location = random.choice(self.locations)
        end_distance = distance / 111000  # Rough conversion to degrees
        end_location = [
            start_location[0] + random.uniform(-end_distance, end_distance),
            start_location[1] + random.uniform(-end_distance, end_distance)
        ]
        
        # Heart rate based on intensity and fitness
        base_hr = 150 + (1 - fitness_level) * 20  # Less fit = higher HR
        avg_hr = base_hr + random.uniform(-10, 15)
        max_hr = avg_hr + random.uniform(10, 30)
        
        # Generate activity name
        activity_names = {
            'Run': ['Morning Run', 'Evening Jog', 'Long Run', 'Speed Work', 'Easy Run', 'Trail Run'],
            'Ride': ['Bike Ride', 'Cycling', 'Long Ride', 'Hills Ride', 'Recovery Ride', 'Group Ride'],
            'Swim': ['Pool Swim', 'Open Water Swim', 'Swim Training', 'Easy Swim'],
            'Hike': ['Morning Hike', 'Trail Hike', 'Mountain Hike', 'Nature Walk']
        }
        name = random.choice(activity_names[activity_type])
        
        # Start date
        start_date = self.base_date + timedelta(days=days_ago)
        
        return MockActivity(
            id=activity_id,
            name=name,
            type=activity_type,
            distance=round(distance, 1),
            moving_time=moving_time,
            elapsed_time=elapsed_time,
            start_date=start_date.isoformat(),
            start_latlng=start_location,
            end_latlng=end_location,
            average_speed=round(speed, 2),
            max_speed=round(speed * 1.3, 2),
            elevation_gain=round(random.uniform(0, distance / 50), 1),  # Rough elevation
            kudos_count=random.randint(0, 15),
            comment_count=random.randint(0, 3),
            athlete_count=1,
            total_elevation_gain=round(random.uniform(0, distance / 50), 1),
            suffer_score=random.randint(20, 200),
            average_heartrate=round(avg_hr, 1),
            max_heartrate=round(max_hr, 1),
            calories=round(distance * 0.8 + moving_time * 0.1, 1)  # Rough calories
        )
    
    def generate_activities(self, count: int = 100, days_back: int = 365) -> List[Dict]:
        """Generate multiple realistic activities"""
        activities = []
        
        # Distribute activities over time (more recent = more activities)
        for i in range(count):
            # Weight towards more recent activities
            days_ago = int(random.weibullvariate(1, 2) * days_back / 3)
            days_ago = min(days_ago, days_back - 1)
            
            activity = self.generate_activity(1000000 + i, days_ago)
            activities.append(asdict(activity))
        
        # Sort by date (most recent first)
        activities.sort(key=lambda x: x['start_date'], reverse=True)
        
        return activities
    
    def generate_athlete_profile(self) -> Dict:
        """Generate realistic athlete profile"""
        return {
            "id": 12345678,
            "username": self.athlete_name.lower().replace(" ", "_"),
            "resource_state": 3,
            "firstname": self.athlete_name.split()[0],
            "lastname": self.athlete_name.split()[-1] if len(self.athlete_name.split()) > 1 else "",
            "bio": f"Fitness enthusiast tracking progress with Pierre MCP Server",
            "city": "San Francisco",
            "state": "California", 
            "country": "United States",
            "sex": "M",
            "premium": True,
            "summit": True,
            "created_at": "2020-01-01T00:00:00Z",
            "updated_at": datetime.now().isoformat(),
            "badge_type_id": 1,
            "weight": 70.0,
            "profile_medium": "https://example.com/avatar.jpg",
            "profile": "https://example.com/avatar_large.jpg",
            "friend": None,
            "follower": None
        }
    
    def generate_stats(self) -> Dict:
        """Generate realistic yearly stats"""
        current_year = datetime.now().year
        activities = self.generate_activities(200, 365)
        
        # Calculate stats from activities
        runs = [a for a in activities if a['type'] == 'Run']
        rides = [a for a in activities if a['type'] == 'Ride']
        swims = [a for a in activities if a['type'] == 'Swim']
        
        def calculate_sport_stats(activities_list):
            if not activities_list:
                return {"count": 0, "distance": 0.0, "moving_time": 0, "elevation_gain": 0.0}
            
            return {
                "count": len(activities_list),
                "distance": round(sum(a['distance'] for a in activities_list), 1),
                "moving_time": sum(a['moving_time'] for a in activities_list),
                "elevation_gain": round(sum(a['elevation_gain'] for a in activities_list), 1)
            }
        
        return {
            "biggest_ride_distance": max([a['distance'] for a in rides] or [0]),
            "biggest_climb_elevation_gain": max([a['elevation_gain'] for a in activities] or [0]),
            "recent_ride_totals": calculate_sport_stats(rides[-4:]),  # Last 4 weeks
            "recent_run_totals": calculate_sport_stats(runs[-4:]),
            "recent_swim_totals": calculate_sport_stats(swims[-4:]),
            "ytd_ride_totals": calculate_sport_stats(rides),
            "ytd_run_totals": calculate_sport_stats(runs), 
            "ytd_swim_totals": calculate_sport_stats(swims),
            "all_ride_totals": calculate_sport_stats(rides),
            "all_run_totals": calculate_sport_stats(runs),
            "all_swim_totals": calculate_sport_stats(swims)
        }

def main():
    """Demo the mock Strava provider"""
    print("🏃 Mock Strava Provider Demo")
    print("=" * 40)
    
    provider = MockStravaProvider("Demo Athlete")
    
    # Generate sample data
    print("📊 Generating sample data...")
    activities = provider.generate_activities(20, 90)  # 20 activities in last 90 days
    athlete = provider.generate_athlete_profile()
    stats = provider.generate_stats()
    
    print(f"\n👤 Athlete: {athlete['firstname']} {athlete['lastname']}")
    print(f"📍 Location: {athlete['city']}, {athlete['state']}")
    print(f"\n🏃 Recent Activities ({len(activities)}):")
    
    for i, activity in enumerate(activities[:5]):  # Show first 5
        date = datetime.fromisoformat(activity['start_date']).strftime('%Y-%m-%d')
        distance_km = activity['distance'] / 1000
        pace_min_km = (activity['moving_time'] / 60) / distance_km if distance_km > 0 else 0
        
        print(f"  {i+1}. {activity['name']} ({activity['type']})")
        print(f"     📅 {date} | 📏 {distance_km:.1f}km | ⏱️ {pace_min_km:.1f}min/km")
    
    print(f"\n📈 Year-to-Date Stats:")
    print(f"  🏃 Runs: {stats['ytd_run_totals']['count']} activities, {stats['ytd_run_totals']['distance']/1000:.0f}km")
    print(f"  🚴 Rides: {stats['ytd_ride_totals']['count']} activities, {stats['ytd_ride_totals']['distance']/1000:.0f}km")
    print(f"  🏊 Swims: {stats['ytd_swim_totals']['count']} activities, {stats['ytd_swim_totals']['distance']/1000:.1f}km")
    
    # Save sample data to file
    sample_data = {
        'athlete': athlete,
        'activities': activities,
        'stats': stats,
        'generated_at': datetime.now().isoformat()
    }
    
    with open('mock_strava_data.json', 'w') as f:
        json.dump(sample_data, f, indent=2)
    
    print(f"\n💾 Sample data saved to 'mock_strava_data.json'")
    print("✅ Mock provider demo complete!")

if __name__ == "__main__":
    main()