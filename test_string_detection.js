// Test file to validate proper string literal detection
// This should pass - all suspicious patterns are in legitimate string contexts

describe('Error message handling', () => {
  it('should display correct error messages', () => {
    const messages = {
      notImplemented: "This feature is not implemented yet",
      mockData: "Using mock data for testing",
      placeholder: "This is a placeholder value"
    };
    
    expect(messages.notImplemented).toContain("not implemented");
    expect(messages.mockData).toContain("mock");
  });

  it('should handle template literals', () => {
    const template = `
      Error: Feature not implemented
      Status: Using fake data for demo
      TODO: Add real implementation
    `;
    
    expect(template).toBeDefined();
  });
});

// Legitimate production code
class ErrorHandler {
  constructor() {
    this.errorMessages = new Map([
      ['NOT_IMPL', "Feature not implemented"],
      ['MOCK_MODE', 'Running in mock mode'],
      ['PLACEHOLDER', `Placeholder content shown`]
    ]);
  }
  
  getErrorMessage(code) {
    return this.errorMessages.get(code) || "Unknown error";
  }
}

module.exports = ErrorHandler;