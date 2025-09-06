// This test file is in wrong location - should be in tests/ directory
// Testing if pretooluse hook blocks test files outside designated directories

describe('User Service', () => {
    it('should return user data', () => {
        const userData = getUserData(1);
        expect(userData).toBeDefined();
        expect(userData.id).toBe(1);
    });

    it('should handle invalid user id', () => {
        const userData = getUserData(-1);
        expect(userData).toBeNull();
    });
});

function mockUserService() {
    return { id: 1, name: "Test User" };
}