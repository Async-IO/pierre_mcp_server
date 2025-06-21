#!/usr/bin/env python3
"""
A2A Protocol Compliance Validator

This script validates Pierre's A2A implementation against the official
Google A2A specification. Run this to verify compliance before deployment.
"""

import requests
import json
import sys
from typing import Dict, List, Optional


class A2AComplianceValidator:
    """Validates A2A protocol compliance"""
    
    def __init__(self, base_url: str = "http://localhost:8081"):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'User-Agent': 'A2A-Compliance-Validator/1.0'
        })
        
    def validate_all(self) -> bool:
        """Run all compliance validations"""
        print("🔍 A2A Protocol Compliance Validation")
        print("=" * 50)
        
        validators = [
            ("Server Health", self.validate_server_health),
            ("JSON-RPC 2.0 Format", self.validate_jsonrpc_format),
            ("Required Methods", self.validate_required_methods),
            ("Agent Card", self.validate_agent_card),
            ("Error Handling", self.validate_error_handling),
            ("Message Structure", self.validate_message_structure),
            ("Task Management", self.validate_task_management),
            ("Tools Interface", self.validate_tools_interface),
            ("Authentication", self.validate_authentication),
        ]
        
        all_passed = True
        results = []
        
        for name, validator in validators:
            print(f"\n📋 Testing: {name}")
            try:
                result = validator()
                if result:
                    print(f"✅ {name}: PASSED")
                    results.append((name, True, None))
                else:
                    print(f"❌ {name}: FAILED")
                    results.append((name, False, "Validation failed"))
                    all_passed = False
            except Exception as e:
                print(f"❌ {name}: ERROR - {str(e)}")
                results.append((name, False, str(e)))
                all_passed = False
        
        # Summary
        print("\n" + "=" * 50)
        print("📊 COMPLIANCE SUMMARY")
        print("=" * 50)
        
        for name, passed, error in results:
            status = "✅ PASS" if passed else "❌ FAIL"
            print(f"{status} | {name}")
            if error and not passed:
                print(f"     Error: {error}")
        
        print("\n" + "=" * 50)
        if all_passed:
            print("🎉 ALL COMPLIANCE TESTS PASSED!")
            print("✅ Pierre is fully compliant with A2A Protocol v1.0")
        else:
            print("⚠️  SOME COMPLIANCE TESTS FAILED")
            print("❌ Please fix issues before deployment")
        
        return all_passed
    
    def validate_server_health(self) -> bool:
        """Test basic server connectivity"""
        try:
            response = self.session.get(f"{self.base_url}/health", timeout=5)
            return response.status_code == 200
        except:
            return False
    
    def validate_jsonrpc_format(self) -> bool:
        """Validate JSON-RPC 2.0 format compliance"""
        request = {
            "jsonrpc": "2.0",
            "method": "a2a/initialize",
            "id": 1
        }
        
        response = self.session.post(f"{self.base_url}/a2a", json=request)
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        
        # Check required JSON-RPC 2.0 fields
        required_fields = ["jsonrpc", "id"]
        for field in required_fields:
            if field not in data:
                return False
        
        # Check jsonrpc version
        if data.get("jsonrpc") != "2.0":
            return False
            
        # Should have either result or error
        has_result = "result" in data
        has_error = "error" in data
        
        return has_result or has_error
    
    def validate_required_methods(self) -> bool:
        """Test that all required A2A methods are implemented"""
        required_methods = [
            "a2a/initialize",
            "message/send",
            "message/stream", 
            "tasks/create",
            "tasks/get",
            "tasks/cancel",
            "tasks/pushNotificationConfig/set",
            "tools/list",
            "tools/call"
        ]
        
        for method in required_methods:
            request = {
                "jsonrpc": "2.0",
                "method": method,
                "id": 1
            }
            
            response = self.session.post(f"{self.base_url}/a2a", json=request)
            
            if response.status_code != 200:
                return False
                
            data = response.json()
            
            # Method should not return "Method not found" error
            if "error" in data and data["error"].get("code") == -32601:
                print(f"   ❌ Method not implemented: {method}")
                return False
        
        print("   ✅ All required methods implemented")
        return True
    
    def validate_agent_card(self) -> bool:
        """Validate Agent Card structure and content"""
        try:
            response = self.session.get(f"{self.base_url}/a2a/agent-card")
            
            if response.status_code != 200:
                return False
                
            agent_card = response.json()
            
            # Check required fields
            required_fields = [
                "name", "description", "version", 
                "capabilities", "authentication", "tools"
            ]
            
            for field in required_fields:
                if field not in agent_card:
                    print(f"   ❌ Missing required field: {field}")
                    return False
            
            # Validate authentication schemes
            auth = agent_card.get("authentication", {})
            schemes = auth.get("schemes", [])
            
            if not schemes:
                print("   ❌ No authentication schemes defined")
                return False
            
            # Check tools have proper structure
            tools = agent_card.get("tools", [])
            for tool in tools:
                required_tool_fields = ["name", "description", "input_schema", "output_schema"]
                for field in required_tool_fields:
                    if field not in tool:
                        print(f"   ❌ Tool missing field: {field}")
                        return False
            
            print("   ✅ Agent Card structure valid")
            return True
            
        except Exception as e:
            print(f"   ❌ Agent Card validation error: {e}")
            return False
    
    def validate_error_handling(self) -> bool:
        """Test error handling compliance"""
        # Test unknown method
        request = {
            "jsonrpc": "2.0", 
            "method": "unknown/method",
            "id": 1
        }
        
        response = self.session.post(f"{self.base_url}/a2a", json=request)
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        
        # Should return error
        if "error" not in data:
            return False
        
        error = data["error"]
        
        # Should have proper error structure
        if "code" not in error or "message" not in error:
            return False
        
        # Should be "Method not found" error (-32601)
        if error["code"] != -32601:
            return False
        
        print("   ✅ Error handling compliant")
        return True
    
    def validate_message_structure(self) -> bool:
        """Test A2A message structure compliance"""
        # Test message send capability
        message_data = {
            "id": "test-message-123",
            "parts": [
                {"type": "text", "content": "Hello, Agent!"},
                {"type": "data", "content": {"key": "value"}}
            ],
            "metadata": {"source": "compliance_test"}
        }
        
        request = {
            "jsonrpc": "2.0",
            "method": "message/send",
            "params": {"message": message_data},
            "id": 1
        }
        
        response = self.session.post(f"{self.base_url}/a2a", json=request)
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        
        # Should not have method not found error
        if "error" in data and data["error"].get("code") == -32601:
            return False
        
        print("   ✅ Message structure handling works")
        return True
    
    def validate_task_management(self) -> bool:
        """Test task management compliance"""
        # Test task creation
        request = {
            "jsonrpc": "2.0",
            "method": "tasks/create",
            "params": {"task_type": "test_task"},
            "id": 1
        }
        
        response = self.session.post(f"{self.base_url}/a2a", json=request)
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        
        # Should have result with task info
        if "result" not in data:
            return False
        
        result = data["result"]
        
        # Task should have required fields
        required_fields = ["id", "status", "created_at"]
        for field in required_fields:
            if field not in result:
                return False
        
        # Test task cancellation
        task_id = result["id"]
        cancel_request = {
            "jsonrpc": "2.0",
            "method": "tasks/cancel",
            "params": {"task_id": task_id},
            "id": 2
        }
        
        cancel_response = self.session.post(f"{self.base_url}/a2a", json=cancel_request)
        
        if cancel_response.status_code != 200:
            return False
        
        print("   ✅ Task management compliant")
        return True
    
    def validate_tools_interface(self) -> bool:
        """Test tools interface compliance"""
        # Test tools list
        request = {
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }
        
        response = self.session.post(f"{self.base_url}/a2a", json=request)
        
        if response.status_code != 200:
            return False
            
        data = response.json()
        
        if "result" not in data:
            return False
        
        tools = data["result"]
        
        if not isinstance(tools, list):
            return False
        
        # Each tool should have proper structure
        for tool in tools:
            required_fields = ["name", "description", "parameters"]
            for field in required_fields:
                if field not in tool:
                    return False
        
        print("   ✅ Tools interface compliant")
        return True
    
    def validate_authentication(self) -> bool:
        """Test authentication scheme support"""
        try:
            # Get agent card to check auth schemes
            response = self.session.get(f"{self.base_url}/a2a/agent-card")
            
            if response.status_code != 200:
                return False
                
            agent_card = response.json()
            auth = agent_card.get("authentication", {})
            schemes = auth.get("schemes", [])
            
            # Should support at least api-key
            if "api-key" not in schemes:
                print("   ❌ API key authentication not supported")
                return False
            
            # Should have API key configuration
            if "api_key" not in auth:
                print("   ❌ API key configuration missing")
                return False
            
            api_key_config = auth["api_key"]
            required_fields = ["header_name", "registration_url"]
            
            for field in required_fields:
                if field not in api_key_config:
                    print(f"   ❌ API key config missing: {field}")
                    return False
            
            print("   ✅ Authentication schemes compliant")
            return True
            
        except Exception as e:
            print(f"   ❌ Authentication validation error: {e}")
            return False


def main():
    """Run A2A compliance validation"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Validate A2A Protocol Compliance")
    parser.add_argument("--url", default="http://localhost:8081", 
                       help="Base URL of Pierre server (default: http://localhost:8081)")
    
    args = parser.parse_args()
    
    validator = A2AComplianceValidator(args.url)
    success = validator.validate_all()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()