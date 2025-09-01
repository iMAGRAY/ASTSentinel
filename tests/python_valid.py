# Valid Python code with legitimate pass usage
class DataProcessor:
    def __init__(self):
        self.data = []
        self.processed = False
    
    def process_batch(self, items):
        """Process a batch of items"""
        if not items:
            return []
        
        results = []
        for item in items:
            if item is None:
                pass  # Skip None values, legitimate use
            elif isinstance(item, str):
                results.append(item.upper())
            elif isinstance(item, int):
                results.append(item * 2)
            else:
                # Log unsupported type
                print(f"Unsupported type: {type(item)}")
        
        self.processed = True
        return results
    
    def validate_data(self, data):
        """Validate input data"""
        if not data:
            raise ValueError("Data cannot be empty")
        
        if len(data) > 1000:
            raise ValueError("Too much data")
        
        return True

def calculate_metrics(values):
    """Calculate statistical metrics"""
    if not values:
        return {"mean": 0, "sum": 0}
    
    total = sum(values)
    mean = total / len(values)
    
    return {
        "mean": mean,
        "sum": total,
        "count": len(values)
    }