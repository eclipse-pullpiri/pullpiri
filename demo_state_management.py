#!/usr/bin/env python3
"""
Demonstration of Scenario State Management Workflow
Shows the complete state transition flow with detailed logging
"""

import time
import json
from datetime import datetime

def print_header(title, emoji="🔄"):
    print(f"\n{emoji} {title}")
    print("=" * (len(title) + 4))

def print_step(step, description, emoji="📍"):
    print(f"{emoji} {step}: {description}")

def print_state_change(component, scenario, current_state, target_state, details=""):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
    print(f"   📤 StateChange Message:")
    print(f"      • Timestamp: {timestamp}")
    print(f"      • Component: {component}")
    print(f"      • Resource Type: SCENARIO")
    print(f"      • Resource Name: {scenario}")
    print(f"      • Current State: {current_state}")
    print(f"      • Target State: {target_state}")
    print(f"      • Transition ID: {component.lower()}-{target_state}-{int(time.time())}")
    print(f"      • Source: {component.lower()}")
    if details:
        print(f"      • Details: {details}")

def print_etcd_storage(scenario, state):
    print(f"   💾 ETCD Storage:")
    print(f"      • Key: /scenario/{scenario}/state")
    print(f"      • Value: {state}")
    print(f"      • Operation: common::etcd::put()")

def simulate_delay(component, action, duration=0.1):
    print(f"   ⏱️  Processing {action} in {component}...")
    time.sleep(duration)
    print(f"   ✅ {action} completed successfully")

def main():
    print_header("Scenario State Management Workflow Demonstration", "🧪")
    print("This demonstration shows the complete scenario state transition flow:")
    print("idle → waiting → satisfied → allowed/denied → completed")
    print("")
    
    scenario_name = "temperature-alert-scenario"
    print(f"📋 Test Scenario: {scenario_name}")
    print(f"🌡️  Condition: Temperature > 25°C")
    print("")

    # Step 1: FilterGateway - idle → waiting
    print_header("STEP 1: FilterGateway Condition Registration", "🔍")
    print_step("1.1", "Vehicle data received: temperature = 26.5°C")
    print_step("1.2", "Condition evaluation: 26.5 > 25.0 = TRUE")
    print_step("1.3", "Condition satisfied - triggering state change")
    
    print_state_change("FilterGateway", scenario_name, "idle", "waiting", 
                      "Scenario condition satisfied")
    simulate_delay("FilterGateway", "state change notification")
    
    print_step("1.4", "Triggering ActionController via gRPC")
    simulate_delay("FilterGateway", "ActionController trigger")
    print("")

    # Step 2: ActionController - waiting → satisfied
    print_header("STEP 2: ActionController Condition Satisfaction", "🎯")
    print_step("2.1", "Received trigger_action from FilterGateway")
    print_step("2.2", "Processing scenario actions and workloads")
    print_step("2.3", "Condition satisfaction confirmed")
    
    print_state_change("ActionController", scenario_name, "waiting", "satisfied",
                      "ActionController confirmed condition satisfaction")
    simulate_delay("ActionController", "condition satisfaction processing")
    print("")

    # Step 3a: PolicyManager - satisfied → allowed (success case)
    print_header("STEP 3a: PolicyManager Policy Check (Success)", "🛡️")
    print_step("3.1", "Evaluating scenario against policy requirements")
    print_step("3.2", "Policy check result: PASSED")
    print_step("3.3", "Scenario meets all policy requirements")
    
    print_state_change("PolicyManager", scenario_name, "satisfied", "allowed",
                      "Policy requirements satisfied")
    simulate_delay("PolicyManager", "policy validation")
    print("")

    # Step 3b: PolicyManager - satisfied → denied (failure case - alternate scenario)
    print_header("STEP 3b: PolicyManager Policy Check (Failure - Alternate)", "🚫")
    restricted_scenario = "security-restricted-scenario"
    print_step("3.1", f"Evaluating alternate scenario: {restricted_scenario}")
    print_step("3.2", "Policy check result: FAILED")
    print_step("3.3", "Scenario violates security policy")
    
    print_state_change("PolicyManager", restricted_scenario, "satisfied", "denied",
                      "Security policy violation detected")
    simulate_delay("PolicyManager", "policy denial processing")
    print("")

    # Step 4: ActionController - allowed → completed
    print_header("STEP 4: ActionController Processing Completion", "🏁")
    print_step("4.1", "Executing scenario actions (launch/update workloads)")
    print_step("4.2", "All workload operations completed successfully")
    print_step("4.3", "Scenario processing finished")
    
    print_state_change("ActionController", scenario_name, "allowed", "completed",
                      "All scenario actions executed successfully")
    simulate_delay("ActionController", "scenario completion")
    print("")

    # Step 5: StateManager - ETCD Persistence
    print_header("STEP 5: StateManager ETCD Persistence", "💾")
    print_step("5.1", "Processing successful state transitions")
    print_step("5.2", "Saving scenario state to persistent storage")
    
    states_to_save = [
        ("waiting", "Initial condition satisfaction"),
        ("satisfied", "ActionController confirmation"),
        ("allowed", "Policy approval"),
        ("completed", "Final processing completion")
    ]
    
    for state, description in states_to_save:
        print(f"   📝 Saving state: {state} ({description})")
        print_etcd_storage(scenario_name, state)
        time.sleep(0.05)
    
    print("   ✅ All state transitions persisted to ETCD")
    print("")

    # Summary
    print_header("WORKFLOW COMPLETION SUMMARY", "🎉")
    print("✅ All scenario state transitions completed successfully:")
    print("")
    print("   🔄 Complete State Flow:")
    print("   ┌─────────────────────────────────────────────────────────────┐")
    print("   │  idle → waiting → satisfied → allowed → completed          │")
    print("   │                              ↘ denied (alternate path)     │")
    print("   └─────────────────────────────────────────────────────────────┘")
    print("")
    print("   📊 Component Interactions:")
    print("   • FilterGateway:   Condition detection & initial state change")
    print("   • ActionController: Condition confirmation & processing completion")
    print("   • PolicyManager:   Policy validation & approval/denial")
    print("   • StateManager:    State coordination & ETCD persistence")
    print("")
    print("   💾 Persistent Storage:")
    print(f"   • All states saved to ETCD: /scenario/{scenario_name}/state")
    print("   • Complete audit trail with timestamps and transition IDs")
    print("   • Full traceability across all components")
    print("")

    # Log Output Example
    print_header("EXAMPLE LOG OUTPUT", "📝")
    print("Here's what the actual log output would look like when running:")
    print("")
    print("🔄 SCENARIO STATE TRANSITION: FilterGateway Processing")
    print("   📋 Scenario: temperature-alert-scenario")
    print("   🔄 State Change: idle → waiting")
    print("   🔍 Reason: Scenario condition satisfied")
    print("   📤 Sending StateChange to StateManager:")
    print("      • Resource Type: SCENARIO")
    print("      • Resource Name: temperature-alert-scenario")
    print("      • Current State: idle")
    print("      • Target State: waiting")
    print("      • Transition ID: filtergateway-condition-met-1234567890")
    print("      • Source: filtergateway")
    print("   ✅ Successfully notified StateManager: scenario temperature-alert-scenario idle → waiting")
    print("   📤 Triggering ActionController via gRPC...")
    print("   ✅ ActionController triggered successfully")
    print("")
    print("🔄 SCENARIO STATE TRANSITION: ActionController Processing")
    print("   📋 Scenario: temperature-alert-scenario")
    print("   🔄 State Change: waiting → satisfied")
    print("   🔍 Reason: ActionController received trigger_action from FilterGateway")
    print("   ✅ Successfully notified StateManager: scenario temperature-alert-scenario waiting → satisfied")
    print("   🎯 Processing scenario actions...")
    print("")
    print("💾 SCENARIO STATE PERSISTENCE: StateManager ETCD Storage")
    print("   📋 Scenario: temperature-alert-scenario")
    print("   🔄 Final State: completed")
    print("   🔍 Reason: Successful state transition completed")
    print("   📤 Saving to ETCD:")
    print("      • Key: /scenario/temperature-alert-scenario/state")
    print("      • Value: completed")
    print("      • Operation: common::etcd::put()")
    print("   ✅ Successfully saved scenario state to ETCD")

if __name__ == "__main__":
    main()