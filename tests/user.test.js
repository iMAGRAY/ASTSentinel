// Edge case 4: Properly placed test files
const { expect } = require('chai');
const UserService = require('../src/UserService');

describe('UserService Tests', () => {
  let userService;
  let mockDatabase;
  
  beforeEach(() => {
    mockDatabase = {
      query: jest.fn().mockResolvedValue({ id: 1, name: 'Test User' }),
      connect: jest.fn(),
      close: jest.fn()
    };
    
    userService = new UserService(mockDatabase);
  });
  
  describe('getUser method', () => {
    it('should fetch user with valid ID', async () => {
      const result = await userService.getUser(1);
      expect(result.id).to.equal(1);
      expect(mockDatabase.query).to.have.been.calledOnce;
    });
    
    it('should handle fake/invalid user scenarios', async () => {
      mockDatabase.query.mockResolvedValue(null);
      
      await expect(userService.getUser(999))
        .to.be.rejectedWith('User not found');
    });
  });
});