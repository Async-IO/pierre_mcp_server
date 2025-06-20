#!/usr/bin/env python3
"""
Quick Demo Runner for Pierre MCP Server Provisioning Examples

This script provides a simple way to run the provisioning demonstrations
without needing to understand all the details.
"""

import os
import sys
import subprocess
import time

def check_server_running():
    """Check if Pierre MCP server is running on port 8081"""
    try:
        import requests
        response = requests.get('http://localhost:8081/health', timeout=3)
        return response.status_code == 200
    except:
        return False

def start_server_instructions():
    """Show instructions for starting the server"""
    print("🚀 Starting Pierre MCP Server")
    print("=" * 40)
    print("Run this command in another terminal from the project root:")
    print()
    print("   cargo run --bin pierre-mcp-server -- --port 8081")
    print()
    print("Then press Enter to continue...")
    input()

def run_mock_provider_demo():
    """Run the mock Strava provider demo"""
    print("🏃 Running Mock Strava Provider Demo")
    print("=" * 40)
    
    try:
        result = subprocess.run([
            sys.executable, 'mock_strava_provider.py'
        ], check=True, capture_output=False)
        
        print("\n✅ Mock provider demo completed!")
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ Mock provider demo failed: {e}")
        return False

def run_provisioning_demo():
    """Run the complete provisioning demo"""
    print("🏢 Running Complete Provisioning Demo")
    print("=" * 40)
    
    try:
        result = subprocess.run([
            sys.executable, 'provisioning_example.py'
        ], check=True, capture_output=False)
        
        print("\n✅ Provisioning demo completed!")
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ Provisioning demo failed: {e}")
        return False

def main():
    """Main demo runner"""
    print("🎯 Pierre MCP Server - Demo Runner")
    print("=" * 50)
    print()
    
    # Check if server is running
    print("🔍 Checking if Pierre MCP server is running...")
    if not check_server_running():
        print("❌ Server not running on http://localhost:8081")
        start_server_instructions()
        
        # Check again
        if not check_server_running():
            print("❌ Server still not accessible. Please start the server first.")
            sys.exit(1)
    
    print("✅ Server is running!")
    print()
    
    # Run demos
    demos = [
        ("Mock Strava Provider", run_mock_provider_demo),
        ("Complete Provisioning Workflow", run_provisioning_demo)
    ]
    
    for name, demo_func in demos:
        print(f"▶️ Running: {name}")
        print("-" * 50)
        
        success = demo_func()
        
        if success:
            print(f"✅ {name} completed successfully!")
        else:
            print(f"❌ {name} failed!")
            
        print()
        time.sleep(1)
    
    print("🎉 All demos completed!")
    print()
    print("📚 Next steps:")
    print("   • Review the generated files (mock_strava_data.json, etc.)")
    print("   • Check docs/PROVISIONING.md for production setup")
    print("   • Try integrating with your AI assistant")
    print("   • Explore other examples in examples/python/")

if __name__ == "__main__":
    # Change to the provisioning directory
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)
    
    main()