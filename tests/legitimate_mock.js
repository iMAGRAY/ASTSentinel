// Test file with clearly marked mocks - should be ALLOWED

// MOCK: Test implementation for unit tests
function mockUserService() {
    // This is a mock for testing purposes
    return {
        id: 123,
        name: "Test User",
        email: "test@example.com"
    };
}

// EXAMPLE: Demonstration code as requested by user
function exampleAuthHandler() {
    // EXAMPLE: This shows the auth flow structure
    return {
        isAuthenticated: true,
        token: "example-token-123"
    };
}

// Proper error handling - not suppressing
async function safeOperation() {
    try {
        const result = await riskyOperation();
        return result;
    } catch (error) {
        console.error('Operation failed:', error);
        throw error; // Re-throwing the error
    }
}

// PLACEHOLDER: Clearly marked temporary implementation
function getConfig() {
    // TODO: Load from config file
    // PLACEHOLDER: Using defaults until config system is ready
    console.warn("Using placeholder configuration");
    return {
        apiUrl: "http://localhost:3000",
        timeout: 5000
    };
}

module.exports = {
    mockUserService,
    exampleAuthHandler,
    safeOperation,
    getConfig
};