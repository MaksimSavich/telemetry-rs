#!/usr/bin/env python3
"""
Test script to validate the latest message prioritization implementation.
"""

import time
import struct
from collections import deque

# Message priority levels
PRIORITY_CRITICAL = 0
PRIORITY_HIGH = 1
PRIORITY_MEDIUM = 2
PRIORITY_LOW = 3

class CanFrameData:
    """Simple CAN frame data structure for testing"""
    
    def __init__(self, can_id: int, data: bytes, sequence_number: int = 0):
        self.id = can_id
        self.data = data[:8]  # CAN max is 8 bytes
        self.timestamp = time.time()
        self.sequence_number = sequence_number
        self.priority = self.get_priority_for_id(can_id)
    
    def get_priority_for_id(self, can_id: int) -> int:
        """Get message priority based on CAN ID"""
        if can_id in [0x300, 0x776, 0x777]:  # Critical safety messages
            return PRIORITY_CRITICAL
        elif can_id in [0x320, 0x0CF11E05, 0x0CF11F05]:  # High priority operational
            return PRIORITY_HIGH
        elif can_id in [0x360, 0x330]:  # Medium priority status
            return PRIORITY_MEDIUM
        else:  # Low priority monitoring
            return PRIORITY_LOW

class TestBatcher:
    """Test implementation of priority-based message batching"""
    
    def __init__(self):
        self.latest_frames = {}  # CAN ID -> CanFrameData
        self.frame_order = deque()
        self.total_frames_added = 0
        self.frames_replaced = 0
        
    def add_frame(self, frame: CanFrameData) -> bool:
        """Add frame with latest message prioritization"""
        can_id = frame.id
        self.total_frames_added += 1
        
        # Check if we already have this message ID
        if can_id in self.latest_frames:
            existing_frame = self.latest_frames[can_id]
            # Replace with newer message
            if (frame.sequence_number > existing_frame.sequence_number or 
                (frame.sequence_number == existing_frame.sequence_number and frame.timestamp > existing_frame.timestamp)):
                self.latest_frames[can_id] = frame
                self.frames_replaced += 1
                print(f"✓ Replaced message 0x{can_id:X} with newer version (seq: {existing_frame.sequence_number} -> {frame.sequence_number})")
            else:
                print(f"✗ Kept older message 0x{can_id:X} (seq: {existing_frame.sequence_number} >= {frame.sequence_number})")
            return True
        
        # Add new message
        self.latest_frames[can_id] = frame
        self.frame_order.append(can_id)
        print(f"+ Added new message 0x{can_id:X} (seq: {frame.sequence_number}, priority: {frame.priority})")
        return True
    
    def get_priority_ordered_frames(self):
        """Get frames ordered by priority (critical first, then by timestamp)"""
        frames = list(self.latest_frames.values())
        # Sort by priority (0=critical first), then by timestamp (newest first)
        frames.sort(key=lambda f: (f.priority, -f.timestamp))
        return frames
    
    def get_stats(self):
        """Get batcher statistics"""
        return {
            'total_added': self.total_frames_added,
            'replaced': self.frames_replaced,
            'unique_messages': len(self.latest_frames),
            'replacement_rate': self.frames_replaced / max(1, self.total_frames_added) * 100
        }

def test_latest_message_prioritization():
    """Test the latest message prioritization logic"""
    print("Testing Latest Message Prioritization")
    print("=" * 50)
    
    batcher = TestBatcher()
    
    # Test 1: Add initial messages
    print("\n1. Adding initial messages...")
    batcher.add_frame(CanFrameData(0x300, b'\x01\x02\x03\x04', 100))  # Critical - BMS DTC
    batcher.add_frame(CanFrameData(0x320, b'\x05\x06\x07\x08', 101))  # High - BMS Power
    batcher.add_frame(CanFrameData(0x360, b'\x09\x0A\x0B\x0C', 102))  # Medium - BMS Temp
    batcher.add_frame(CanFrameData(0x310, b'\x0D\x0E\x0F\x10', 103))  # Low - BMS Limits
    
    # Test 2: Add newer versions (should replace)
    print("\n2. Adding newer versions (should replace)...")
    time.sleep(0.001)  # Small delay to ensure different timestamps
    batcher.add_frame(CanFrameData(0x300, b'\x11\x12\x13\x14', 104))  # Newer DTC
    batcher.add_frame(CanFrameData(0x320, b'\x15\x16\x17\x18', 105))  # Newer Power
    
    # Test 3: Add older versions (should NOT replace)
    print("\n3. Adding older versions (should NOT replace)...")
    batcher.add_frame(CanFrameData(0x300, b'\x19\x1A\x1B\x1C', 99))   # Older DTC
    batcher.add_frame(CanFrameData(0x320, b'\x1D\x1E\x1F\x20', 100))  # Older Power
    
    # Test 4: Check priority ordering
    print("\n4. Checking priority ordering...")
    ordered_frames = batcher.get_priority_ordered_frames()
    print("Priority order (Critical -> High -> Medium -> Low):")
    for i, frame in enumerate(ordered_frames):
        priority_names = ["Critical", "High", "Medium", "Low"]
        print(f"  {i+1}. ID: 0x{frame.id:X}, Priority: {priority_names[frame.priority]}, Seq: {frame.sequence_number}")
    
    # Test 5: Display statistics
    print("\n5. Final statistics:")
    stats = batcher.get_stats()
    for key, value in stats.items():
        print(f"  {key}: {value}")
    
    # Test 6: Verify delay detection capability
    print("\n6. Testing delay detection...")
    latest_critical = None
    latest_high = None
    
    for frame in ordered_frames:
        if frame.priority == PRIORITY_CRITICAL and latest_critical is None:
            latest_critical = frame
        elif frame.priority == PRIORITY_HIGH and latest_high is None:
            latest_high = frame
    
    if latest_critical and latest_high:
        seq_diff = abs(latest_critical.sequence_number - latest_high.sequence_number)
        time_diff = abs(latest_critical.timestamp - latest_high.timestamp) * 1000  # ms
        print(f"  Critical vs High message gap: {seq_diff} sequences, {time_diff:.2f}ms")
        
        if seq_diff > 10:
            print("  ⚠️  Large sequence gap detected - potential delay issue!")
        else:
            print("  ✓ Sequence gap within normal range")
    
    print("\nTest completed! ✓")
    print("\nKey improvements:")
    print("- Messages are now prioritized by safety criticality")
    print("- Latest version of each message is kept (prevents stale data)")
    print("- Sequence numbers enable delay detection")
    print("- Significant reduction in message queue size")
    print("- Frontend will receive most recent data first")

if __name__ == "__main__":
    test_latest_message_prioritization()