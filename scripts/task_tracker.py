#!/usr/bin/env python3
"""
Kitchen Management System - Task Tracker
Simple script to track progress on the 24 enhancement tasks
"""

import json
import datetime
from pathlib import Path

class KitchenTaskTracker:
    def __init__(self):
        self.tasks_file = Path("task_progress.json")
        self.tasks = self.load_tasks()
    
    def load_tasks(self):
        """Load tasks from JSON file or create default structure"""
        if self.tasks_file.exists():
            with open(self.tasks_file, 'r') as f:
                return json.load(f)
        
        # Default task structure
        return {
            "project_info": {
                "name": "Kitchen Management System Enhancement",
                "start_date": datetime.date.today().isoformat(),
                "total_tasks": 24,
                "completed": 0,
                "in_progress": 0
            },
            "tasks": {
                # Testing Tasks
                "T1": {"name": "Unit tests for core business logic", "status": "pending", "priority": "high", "days": 4},
                "T2": {"name": "Integration tests for API endpoints", "status": "pending", "priority": "high", "days": 5},
                "T3": {"name": "End-to-end test scenarios", "status": "pending", "priority": "medium", "days": 6},
                
                # Documentation Tasks
                "D1": {"name": "Inline documentation for APIs", "status": "pending", "priority": "high", "days": 3},
                "D2": {"name": "Architecture Decision Records", "status": "pending", "priority": "medium", "days": 4},
                "D3": {"name": "API usage examples and README", "status": "pending", "priority": "medium", "days": 3},
                
                # Performance Tasks
                "P1": {"name": "gRPC connection pooling", "status": "pending", "priority": "high", "days": 4},
                "P2": {"name": "Redis caching integration", "status": "pending", "priority": "high", "days": 5},
                "P3": {"name": "Request batching for bulk operations", "status": "pending", "priority": "medium", "days": 4},
                
                # Security Tasks
                "S1": {"name": "Request/response validation middleware", "status": "pending", "priority": "high", "days": 4},
                "S2": {"name": "Rate limiting with Redis", "status": "pending", "priority": "high", "days": 3},
                "S3": {"name": "Security headers and CORS", "status": "pending", "priority": "medium", "days": 2},
                
                # Core Features
                "CF1": {"name": "Menu management system", "status": "pending", "priority": "critical", "days": 9},
                "CF2": {"name": "Inventory tracking system", "status": "pending", "priority": "critical", "days": 11},
                "CF3": {"name": "Order management workflow", "status": "pending", "priority": "critical", "days": 14},
                "CF4": {"name": "Staff management with RBAC", "status": "pending", "priority": "high", "days": 7},
                "CF5": {"name": "Table/reservation system", "status": "pending", "priority": "high", "days": 9},
                
                # Technical Enhancements
                "TE1": {"name": "WebSockets for real-time updates", "status": "pending", "priority": "high", "days": 6},
                "TE2": {"name": "Mobile app API endpoints", "status": "pending", "priority": "high", "days": 7},
                "TE3": {"name": "Kitchen display system", "status": "pending", "priority": "high", "days": 8},
                "TE4": {"name": "Reporting and analytics dashboard", "status": "pending", "priority": "medium", "days": 9},
                
                # Operational Improvements
                "OI1": {"name": "CI/CD pipeline with GitHub Actions", "status": "pending", "priority": "high", "days": 4},
                "OI2": {"name": "Feature flags for gradual rollouts", "status": "pending", "priority": "medium", "days": 5},
                "OI3": {"name": "Monitoring with Prometheus/Grafana", "status": "pending", "priority": "high", "days": 6}
            }
        }
    
    def save_tasks(self):
        """Save tasks to JSON file"""
        with open(self.tasks_file, 'w') as f:
            json.dump(self.tasks, f, indent=2)
    
    def update_task_status(self, task_id, status, assignee=None, notes=None):
        """Update task status"""
        if task_id in self.tasks["tasks"]:
            self.tasks["tasks"][task_id]["status"] = status
            self.tasks["tasks"][task_id]["last_updated"] = datetime.date.today().isoformat()
            
            if assignee:
                self.tasks["tasks"][task_id]["assignee"] = assignee
            if notes:
                self.tasks["tasks"][task_id]["notes"] = notes
            
            self.update_project_stats()
            self.save_tasks()
            print(f"✅ Updated {task_id}: {status}")
        else:
            print(f"❌ Task {task_id} not found")
    
    def update_project_stats(self):
        """Update project statistics"""
        completed = sum(1 for task in self.tasks["tasks"].values() if task["status"] == "completed")
        in_progress = sum(1 for task in self.tasks["tasks"].values() if task["status"] == "in_progress")
        
        self.tasks["project_info"]["completed"] = completed
        self.tasks["project_info"]["in_progress"] = in_progress
    
    def show_status(self):
        """Display current project status"""
        info = self.tasks["project_info"]
        print(f"\n🍽️  {info['name']}")
        print("=" * 50)
        print(f"📅 Start Date: {info['start_date']}")
        print(f"📊 Progress: {info['completed']}/{info['total_tasks']} completed")
        print(f"⚡ In Progress: {info['in_progress']}")
        print(f"⏳ Pending: {info['total_tasks'] - info['completed'] - info['in_progress']}")
        
        completion_rate = (info['completed'] / info['total_tasks']) * 100
        print(f"📈 Completion Rate: {completion_rate:.1f}%")
        
        # Show progress bar
        completed_bars = int(completion_rate / 5)  # 5% per bar
        progress_bar = "█" * completed_bars + "░" * (20 - completed_bars)
        print(f"📊 Progress: [{progress_bar}] {completion_rate:.1f}%")
    
    def show_tasks_by_category(self):
        """Show tasks organized by category"""
        categories = {
            "T": "🧪 Testing",
            "D": "📚 Documentation", 
            "P": "⚡ Performance",
            "S": "🔒 Security",
            "CF": "🍽️ Core Features",
            "TE": "🔧 Technical Enhancements",
            "OI": "🚀 Operational Improvements"
        }
        
        for prefix, category in categories.items():
            print(f"\n{category}")
            print("-" * 30)
            
            category_tasks = {k: v for k, v in self.tasks["tasks"].items() if k.startswith(prefix)}
            for task_id, task in category_tasks.items():
                status_emoji = {
                    "pending": "🔴",
                    "in_progress": "🟡", 
                    "completed": "🟢"
                }.get(task["status"], "⚪")
                
                priority_emoji = {
                    "critical": "🚨",
                    "high": "🔥",
                    "medium": "📝",
                    "low": "💡"
                }.get(task["priority"], "")
                
                print(f"  {status_emoji} {task_id}: {task['name']} {priority_emoji} ({task['days']} days)")
    
    def show_next_tasks(self, count=5):
        """Show next recommended tasks to work on"""
        print(f"\n🎯 Next {count} Recommended Tasks")
        print("=" * 40)
        
        # Priority order: critical > high > medium > low
        priority_order = {"critical": 0, "high": 1, "medium": 2, "low": 3}
        
        pending_tasks = [
            (task_id, task) for task_id, task in self.tasks["tasks"].items() 
            if task["status"] == "pending"
        ]
        
        # Sort by priority then by estimated days (shorter first)
        pending_tasks.sort(key=lambda x: (priority_order.get(x[1]["priority"], 4), x[1]["days"]))
        
        for i, (task_id, task) in enumerate(pending_tasks[:count], 1):
            priority_emoji = {
                "critical": "🚨",
                "high": "🔥", 
                "medium": "📝",
                "low": "💡"
            }.get(task["priority"], "")
            
            print(f"{i}. {task_id}: {task['name']}")
            print(f"   Priority: {task['priority']} {priority_emoji} | Estimated: {task['days']} days")
            print()

def main():
    tracker = KitchenTaskTracker()
    
    print("🍽️ Kitchen Management System - Task Tracker")
    print("=" * 50)
    
    while True:
        print("\nOptions:")
        print("1. Show project status")
        print("2. Show all tasks by category")
        print("3. Show next recommended tasks")
        print("4. Update task status")
        print("5. Export task summary")
        print("6. Exit")
        
        choice = input("\nSelect option (1-6): ").strip()
        
        if choice == "1":
            tracker.show_status()
            
        elif choice == "2":
            tracker.show_tasks_by_category()
            
        elif choice == "3":
            count = input("How many tasks to show? (default 5): ").strip()
            count = int(count) if count.isdigit() else 5
            tracker.show_next_tasks(count)
            
        elif choice == "4":
            task_id = input("Enter task ID (e.g., T1, CF2): ").strip().upper()
            print("Status options: pending, in_progress, completed")
            status = input("Enter new status: ").strip().lower()
            assignee = input("Enter assignee (optional): ").strip()
            notes = input("Enter notes (optional): ").strip()
            
            tracker.update_task_status(task_id, status, assignee or None, notes or None)
            
        elif choice == "5":
            # Export summary
            with open("task_summary.txt", "w") as f:
                f.write("Kitchen Management System - Task Summary\n")
                f.write("=" * 50 + "\n\n")
                
                info = tracker.tasks["project_info"]
                f.write(f"Project: {info['name']}\n")
                f.write(f"Start Date: {info['start_date']}\n")
                f.write(f"Total Tasks: {info['total_tasks']}\n")
                f.write(f"Completed: {info['completed']}\n")
                f.write(f"In Progress: {info['in_progress']}\n")
                f.write(f"Pending: {info['total_tasks'] - info['completed'] - info['in_progress']}\n\n")
                
                for task_id, task in tracker.tasks["tasks"].items():
                    f.write(f"{task_id}: {task['name']} - {task['status']} ({task['priority']} priority)\n")
            
            print("✅ Task summary exported to task_summary.txt")
            
        elif choice == "6":
            print("👋 Happy coding!")
            break
            
        else:
            print("❌ Invalid option. Please try again.")

if __name__ == "__main__":
    main()
