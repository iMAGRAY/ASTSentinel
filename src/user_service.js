// Legitimate user service implementation
class UserService {
    constructor(database) {
        this.db = database;
    }

    async getUserById(id) {
        if (!id || typeof id !== 'number') {
            throw new Error('Invalid user ID provided');
        }

        try {
            const user = await this.db.findById('users', id);
            if (!user) {
                return null;
            }
            return {
                id: user.id,
                name: user.name,
                email: user.email,
                createdAt: user.created_at
            };
        } catch (error) {
            console.error('Database error:', error);
            throw new Error('Failed to retrieve user');
        }
    }

    async createUser(userData) {
        // TODO: Add input validation
        try {
            const result = await this.db.insert('users', userData);
            return result;
        } catch (error) {
            console.error('Failed to create user:', error);
            throw error;
        }
    }
}

module.exports = UserService;