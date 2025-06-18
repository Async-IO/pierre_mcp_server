#!/usr/bin/env python3
"""
Demo Runner Script
Easily run MCP and A2A demonstrations
"""

import sys
import subprocess
import os
from pathlib import Path

def run_command(cmd, description):
    """Run a command and show results"""
    print(f"\n🔄 {description}")
    print("-" * 50)
    
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=300)
        
        if result.returncode == 0:
            print("✅ Success!")
            if result.stdout:
                print(result.stdout)
        else:
            print("❌ Failed!")
            if result.stderr:
                print(f"Error: {result.stderr}")
            if result.stdout:
                print(f"Output: {result.stdout}")
        
        return result.returncode == 0
        
    except subprocess.TimeoutExpired:
        print("⏰ Timeout - Demo took too long")
        return False
    except Exception as e:
        print(f"💥 Exception: {e}")
        return False

def main():
    """Run demonstration scripts"""
    
    print("🚀 PIERRE FITNESS API - DEMO RUNNER")
    print("=" * 55)
    print("Choose a demonstration to run:")
    print()
    print("1. MCP Investor Demo (Real-time Analysis)")
    print("2. A2A Enterprise Demo (API Integration)")
    print("3. MCP Data Collection Test")
    print("4. A2A API Client Test")
    print("5. Run All Demos")
    print("6. Exit")
    
    choice = input("\nEnter your choice (1-6): ").strip()
    
    # Set working directory to examples
    examples_dir = Path(__file__).parent
    os.chdir(examples_dir)
    
    if choice == "1":
        print("\n🎬 Running MCP Investor Demonstration...")
        success = run_command("python3 python/mcp/investor_demo.py", "MCP Investor Demo")
        
    elif choice == "2":
        print("\n🏢 Running A2A Enterprise Demonstration...")
        success = run_command("python3 python/a2a/enterprise_demo.py", "A2A Enterprise Demo")
        
    elif choice == "3":
        print("\n📊 Testing MCP Data Collection...")
        success = run_command("python3 python/mcp/data_collection.py", "MCP Data Collection Test")
        
    elif choice == "4":
        print("\n🔧 Testing A2A API Client...")
        success = run_command("python3 python/a2a/api_client.py", "A2A API Client Test")
        
    elif choice == "5":
        print("\n🎪 Running All Demonstrations...")
        
        demos = [
            ("python3 python/mcp/data_collection.py", "MCP Data Collection"),
            ("python3 python/a2a/api_client.py", "A2A API Client"), 
            ("python3 python/mcp/investor_demo.py", "MCP Investor Demo"),
            ("python3 python/a2a/enterprise_demo.py", "A2A Enterprise Demo")
        ]
        
        results = []
        for cmd, desc in demos:
            results.append(run_command(cmd, desc))
        
        print(f"\n📊 DEMO RESULTS SUMMARY:")
        for i, (cmd, desc) in enumerate(demos):
            status = "✅ PASSED" if results[i] else "❌ FAILED"
            print(f"   • {desc}: {status}")
        
        success = all(results)
        
    elif choice == "6":
        print("👋 Goodbye!")
        return
        
    else:
        print("❌ Invalid choice")
        return
    
    if success:
        print(f"\n🎉 Demo completed successfully!")
    else:
        print(f"\n💥 Demo encountered issues")
        print("🔧 Check server status and try again")

if __name__ == "__main__":
    main()