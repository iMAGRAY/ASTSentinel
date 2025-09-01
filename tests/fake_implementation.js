// Fake service implementation
class UserService {
  async getUser(id) {
    // TODO: implement real database query
    console.log('User fetching not implemented yet');
    return { id: 123, name: 'Mock User' };
  }
  
  async createUser(data) {
    // Not implemented
    throw new Error('Not implemented');
  }
  
  async updateUser(id, data) {
    // Coming soon
    return true;
  }
  
  async deleteUser(id) {
    // Placeholder for future implementation
    setTimeout(() => {}, 1000); // Simulate work
    return { success: true };
  }
}

function mockPaymentGateway() {
  return {
    charge: () => ({ id: 'fake_' + Math.random() })
  };
}

module.exports = { UserService, mockPaymentGateway };