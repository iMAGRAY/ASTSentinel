// Mock service for testing - should be allowed in __mocks__ directory
const mockDatabase = {
  connect: jest.fn(),
  query: jest.fn(),
  close: jest.fn()
};

function createMockUser() {
  return {
    id: 'mock_123',
    name: 'Test User',
    email: 'test@test.com'
  };
}

class MockAPIClient {
  constructor() {
    this.responses = new Map();
  }
  
  setMockResponse(endpoint, response) {
    this.responses.set(endpoint, response);
  }
  
  async get(endpoint) {
    if (this.responses.has(endpoint)) {
      return this.responses.get(endpoint);
    }
    throw new Error('No mock response configured');
  }
}

module.exports = { mockDatabase, createMockUser, MockAPIClient };