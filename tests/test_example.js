// Test file in correct directory - should be allowed
// Updated to verify hooks work correctly
describe('Calculator Test Suite', () => {
    it('should add two numbers correctly', () => {
        expect(add(2, 3)).toBe(5);
    });
    
    it('should subtract numbers', () => {
        expect(subtract(10, 4)).toBe(6);
    });
    
    beforeEach(() => {
        // Setup test environment
        console.log('Starting test');
    });
});