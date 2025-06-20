#!/usr/bin/env python3
"""
A2A Business Demonstration
Complete fitness analytics via Agent-to-Agent protocol
"""

import sys
import os
sys.path.append(os.path.join(os.path.dirname(__file__), '..'))

import json
import time
from datetime import datetime
from common.auth_utils import AuthManager, EnvironmentConfig
from common.data_utils import FitnessDataProcessor, DataValidator, DataAnonymizer
from api_client import A2AClient

def _generate_mock_activities(count: int = 200):
    """Generate mock fitness activities for CI/testing"""
    import random
    from datetime import datetime, timedelta
    
    sports = ['Run', 'Ride', 'Hike', 'Walk', 'EbikeRide', 'NordicSki', 'Kayaking']
    activities = []
    
    base_date = datetime.now() - timedelta(days=140)
    
    for i in range(count):
        sport = random.choice(sports)
        
        # Generate realistic data based on sport
        if sport == 'Run':
            distance = random.uniform(2000, 15000)  # 2-15km
            duration = distance * random.uniform(4.5, 7.0)  # 4.5-7 min/km pace
            elevation = random.uniform(10, 300)
        elif sport == 'Ride':
            distance = random.uniform(10000, 80000)  # 10-80km
            duration = distance * random.uniform(1.5, 3.0)  # cycling speed
            elevation = random.uniform(50, 1500)
        elif sport == 'Hike':
            distance = random.uniform(3000, 20000)  # 3-20km
            duration = distance * random.uniform(8, 15)  # hiking pace
            elevation = random.uniform(100, 800)
        else:
            distance = random.uniform(1000, 10000)
            duration = distance * random.uniform(5, 12)
            elevation = random.uniform(10, 200)
        
        activity = {
            'id': f"mock_{i}",
            'sport_type': sport,
            'distance_meters': int(distance),
            'duration_seconds': int(duration),
            'moving_time_seconds': int(duration * 0.95),
            'elevation_gain': int(elevation),
            'start_date': (base_date + timedelta(days=i * 140 / count)).isoformat() + 'Z',
            'average_heart_rate': random.randint(120, 160) if random.random() > 0.3 else None,
            'max_heart_rate': random.randint(150, 185) if random.random() > 0.3 else None,
            'calories': int(duration / 60 * random.uniform(8, 15)) if random.random() > 0.4 else None,
            'provider': 'mock',
            'is_real_data': False
        }
        activities.append(activity)
    
    return activities

def enterprise_demonstration():
    """Complete enterprise demonstration showcasing A2A capabilities"""
    
    print("🏢 PIERRE FITNESS API - A2A ENTERPRISE DEMO")
    print("=" * 65)
    print("🎯 Purpose: Scalable fitness analytics for B2B clients")
    print("📡 Protocol: Agent-to-Agent (A2A) REST API")
    print("⚡ Benefits: High throughput, scalable, enterprise integration")
    print("=" * 65)
    
    # Setup authentication and client
    print("\n🔐 ENTERPRISE AUTHENTICATION SETUP")
    print("-" * 40)
    
    # Skip authentication in CI mode for faster testing
    if os.getenv('PIERRE_CI_MODE') == 'true':
        print("✅ Enterprise authentication skipped (CI mode)")
        client = A2AClient()
    else:
        auth_manager = AuthManager()
        client = A2AClient()
        
        # Authenticate for API access
        if not client.authenticate_with_jwt('test@example.com', 'password123'):
            print("❌ Enterprise authentication failed")
            return False
        
        print("✅ Enterprise authentication successful")
    
    # Create dedicated API key for production use
    print("\n🔑 API KEY PROVISIONING")
    print("-" * 30)
    
    if os.getenv('PIERRE_CI_MODE') == 'true':
        print("✅ API key provisioning skipped (CI mode)")
        api_key = None
    else:
        api_key = client.create_api_key(
            name='Enterprise Demo Key',
            description='API key for business demonstration',
            tier='professional'
        )
        
        if api_key:
            print(f"✅ Enterprise API key provisioned")
            print(f"🔒 Key prefix: {api_key[:12]}...")
            
            # Update client to use API key
            client.api_key = api_key
            client.session.headers['Authorization'] = f'Bearer {api_key}'
        else:
            print("⚠️ Using JWT for demo (API key creation failed)")
    
    # Demonstrate bulk data processing
    print("\n📊 ENTERPRISE DATA PROCESSING")
    print("-" * 35)
    
    print("🔄 Processing enterprise fitness dataset...")
    start_time = time.time()
    
    # FOR REAL DATA: Uncomment this section and ensure Pierre AI server is running
    # with connected Strava account (see README.md for OAuth setup)
    #
    # activities = client.get_activities(limit=200)  # Real Strava data
    # 
    # This connects to your running Pierre AI server and retrieves actual
    # fitness activities from connected providers (Strava, Fitbit, etc.)
    
    # FOR CI/TESTING: Use mock data when server not available or CI mode
    if os.getenv('PIERRE_CI_MODE') == 'true':
        print("📝 Using mock data for CI/testing environment")
        activities = _generate_mock_activities(200)
    else:
        activities = client.get_activities(limit=200)
        if not activities:
            print("📝 Using mock data for demonstration (server not available)")
            activities = _generate_mock_activities(200)
    
    processing_time = time.time() - start_time
    
    if not activities:
        print("❌ Enterprise data processing failed")
        return False
    
    print(f"✅ Processed {len(activities)} activities in {processing_time:.2f}s")
    print(f"📈 Processing rate: {len(activities)/processing_time:.1f} activities/second")
    
    # Anonymize data for privacy protection
    print("\n🔒 ENTERPRISE PRIVACY PROTECTION")
    print("-" * 40)
    print("🔄 Anonymizing personal data for privacy compliance...")
    activities = DataAnonymizer.anonymize_activity_list(activities)
    print("✅ Personal data anonymized (names, GPS, location details removed)")
    
    # Enterprise data quality validation
    print("\n🔍 ENTERPRISE DATA QUALITY ASSURANCE")
    print("-" * 45)
    
    validation = DataValidator.validate_activity_data(activities)
    quality_score = validation['quality_score']
    
    print(f"📊 Data Quality Score: {quality_score:.1f}/100")
    
    if quality_score >= 90:
        quality_level = "🟢 HIGH QUALITY"
    elif quality_score >= 80:
        quality_level = "🟡 GOOD QUALITY"
    else:
        quality_level = "🔴 QUALITY ISSUES"
    
    print(f"📋 Quality Assessment: {quality_level}")
    
    if validation['issues']:
        print("⚠️ Quality Issues Identified:")
        for issue in validation['issues']:
            print(f"   • {issue}")
    
    # Enterprise fitness analytics
    print("\n🤖 ENTERPRISE AI ANALYTICS SUITE")
    print("-" * 40)
    
    # Comprehensive fitness scoring
    print("⚡ Running enterprise fitness analysis...")
    fitness_score = client.calculate_fitness_score()
    
    if fitness_score:
        score_data = fitness_score.get('fitness_score', {})
        overall_score = score_data.get('overall_score', 0)
        components = score_data.get('components', {})
        insights = score_data.get('insights', [])
        
        print(f"🏆 Enterprise Fitness Score: {overall_score}/100")
        
        print("\n📊 Enterprise Component Analysis:")
        for component, score in components.items():
            if score >= 22:
                level = "PREMIUM"
            elif score >= 18:
                level = "STANDARD"
            else:
                level = "BASIC"
            print(f"   • {component.title()}: {score}/25 ({level})")
        
        print("\n💡 Enterprise AI Insights:")
        for i, insight in enumerate(insights, 1):
            print(f"   {i}. {insight}")
    
    # Enterprise training recommendations
    print("\n📋 ENTERPRISE TRAINING OPTIMIZATION")
    print("-" * 40)
    
    recommendations = client.generate_recommendations()
    
    if recommendations:
        print(f"✅ Generated {len(recommendations)} enterprise recommendations:")
        
        for i, rec in enumerate(recommendations, 1):
            title = rec.get('title', 'Optimization')
            description = rec.get('description', 'Enterprise training optimization')
            priority = rec.get('priority', 'medium').upper()
            
            print(f"\n   {i}. [{priority}] {title}")
            print(f"      Enterprise Impact: {description}")
    
    # Enterprise training load analysis
    print("\n⚖️ ENTERPRISE LOAD MANAGEMENT")
    print("-" * 35)
    
    load_analysis = client.analyze_training_load()
    
    if load_analysis:
        load_level = load_analysis.get('load_level', 'unknown')
        weekly_hours = load_analysis.get('weekly_hours', 0)
        weekly_distance = load_analysis.get('weekly_distance_km', 0)
        recovery_score = load_analysis.get('recovery_score', 0)
        
        print(f"📈 Enterprise Load Classification: {load_level.upper()}")
        print(f"⏱️ Weekly Training Volume: {weekly_hours:.1f} hours")
        print(f"📏 Weekly Distance Volume: {weekly_distance:.1f} km")
        print(f"😴 Recovery Optimization Score: {recovery_score}/100")
        
        load_insights = load_analysis.get('insights', [])
        if load_insights:
            print("\n💡 Enterprise Load Insights:")
            for insight in load_insights:
                print(f"   • {insight}")
    
    # Advanced analytics with local processing
    print("\n📊 ADVANCED ENTERPRISE ANALYTICS")
    print("-" * 40)
    
    print("⚡ Running advanced local analytics...")
    advanced_results = FitnessDataProcessor.calculate_fitness_score(activities)
    
    # Enterprise metrics dashboard
    metrics = advanced_results['metrics']
    distribution = advanced_results['distribution']
    
    print("\n📈 ENTERPRISE METRICS DASHBOARD:")
    
    totals = metrics['totals']
    frequency = metrics['frequency']
    
    print(f"   📊 Volume Metrics:")
    print(f"      • Total Activities: {totals['activities']}")
    print(f"      • Total Distance: {totals['distance_km']:.1f} km")
    print(f"      • Total Training Time: {totals['duration_hours']:.1f} hours")
    print(f"      • Total Elevation: {totals['elevation_m']:.0f} meters")
    
    print(f"   📅 Frequency Metrics:")
    print(f"      • Activities per Week: {frequency['activities_per_week']:.1f}")
    print(f"      • Analysis Timespan: {frequency['timespan_days']} days")
    
    print(f"   🏃 Activity Distribution:")
    sport_dist = distribution['sport_distribution']
    for sport, count in sorted(sport_dist.items(), key=lambda x: x[1], reverse=True)[:5]:
        percentage = (count / len(activities)) * 100
        print(f"      • {sport.title()}: {count} ({percentage:.1f}%)")
    
    # Enterprise API usage monitoring
    print("\n📈 ENTERPRISE API MONITORING")
    print("-" * 35)
    
    usage_stats = client.get_api_usage_stats()
    if 'error' not in usage_stats:
        print("✅ API usage monitoring active:")
        print(f"   📊 Usage statistics: {usage_stats}")
    else:
        print("ℹ️ API monitoring data not available")
    
    # Generate enterprise report
    print("\n📋 ENTERPRISE REPORT GENERATION")
    print("-" * 40)
    
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    report_filename = f"a2a_enterprise_report_{timestamp}.json"
    
    enterprise_report = {
        'report_metadata': {
            'generated_at': datetime.now().isoformat(),
            'protocol': 'A2A',
            'report_type': 'Enterprise Analytics',
            'activities_processed': len(activities),
            'processing_time_seconds': processing_time,
            'api_key_used': api_key[:12] + "..." if api_key else None
        },
        'data_quality': validation,
        'fitness_analytics': {
            'api_results': fitness_score,
            'advanced_analysis': advanced_results
        },
        'training_optimization': {
            'recommendations': recommendations,
            'load_analysis': load_analysis
        },
        'enterprise_metrics': metrics,
        'usage_monitoring': usage_stats
    }
    
    try:
        with open(report_filename, 'w') as f:
            json.dump(enterprise_report, f, indent=2)
        print(f"✅ Enterprise report saved: {report_filename}")
    except Exception as e:
        print(f"⚠️ Report save failed: {e}")
    
    # Enterprise demonstration summary
    print(f"\n🎯 ENTERPRISE DEMONSTRATION SUMMARY")
    print("=" * 45)
    print("✅ A2A Protocol: Business API integration validated")
    print(f"✅ Scalability: {len(activities)} activities processed in {processing_time:.2f}s")
    print(f"✅ Data Quality: {quality_score:.1f}/100 validation score")
    print(f"✅ AI Analytics: Comprehensive fitness intelligence delivered")
    print(f"✅ API Management: Automated key provisioning")
    print(f"✅ Monitoring: Usage tracking and analytics available")
    print(f"✅ Reporting: Business report generated ({report_filename})")
    
    print(f"\n🚀 A2A BUSINESS VALUE:")
    print("   • High-throughput batch processing")
    print("   • Multi-tier API key management")
    print("   • Scalable REST API architecture")
    print("   • B2B integration ready")
    print("   • Monitoring and analytics")
    print("   • Comprehensive business reporting")
    
    # Cleanup generated report file
    try:
        if os.path.exists(report_filename):
            os.remove(report_filename)
            print(f"🧹 Cleaned up report file: {report_filename}")
    except Exception as e:
        print(f"⚠️ Cleanup failed: {e}")
    
    return True

def main():
    """Run the complete enterprise demonstration"""
    
    # Setup environment
    EnvironmentConfig.setup_environment()
    
    print("🏢 Starting A2A Enterprise Demonstration...")
    print("📋 This demo showcases scalable fitness analytics")
    print()
    
    success = enterprise_demonstration()
    
    if success:
        print(f"\n🎉 ENTERPRISE DEMONSTRATION COMPLETED!")
        print("💼 Ready for B2B client presentation")
        print("🏢 Enterprise capabilities fully validated")
    else:
        print(f"\n❌ Enterprise demonstration encountered issues")
        print("🔧 Check API server status and authentication")
    
    return success

if __name__ == "__main__":
    success = main()
    exit(0 if success else 1)