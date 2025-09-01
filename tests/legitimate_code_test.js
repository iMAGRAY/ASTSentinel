/**
 * Legitimate production code with proper TODOs and patterns
 * This should NOT trigger false positives
 */

// TODO: (JIRA-1234) Optimize database query performance after v2.0 release
// TODO: #567 Add caching layer when Redis is deployed
// TODO: (@john) Review security implications with the team
// TODO: (2024-03-01) Migrate to new API endpoint

class UserService {
  constructor(database) {
    this.db = database;
    this.cache = new Map();
  }

  // Legitimate validator function that returns boolean
  isValidEmail(email) {
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return emailRegex.test(email);
  }

  // Another validator
  hasPermission(user, action) {
    if (!user || !user.roles) return false;
    return user.roles.includes(action);
  }

  // Legitimate early return pattern
  async getUser(id) {
    if (!id) return null;
    
    // Check cache first
    if (this.cache.has(id)) {
      return this.cache.get(id);
    }
    
    try {
      const user = await this.db.query('SELECT * FROM users WHERE id = ?', [id]);
      if (user) {
        this.cache.set(id, user);
      }
      return user;
    } catch (error) {
      console.error('Database error:', error);
      return null;
    }
  }

  // TODO: Performance optimization - batch queries for multiple users
  async getUsers(ids) {
    // Current implementation - works but could be optimized
    const users = [];
    for (const id of ids) {
      const user = await this.getUser(id);
      if (user) users.push(user);
    }
    return users;
  }

  // Check if user exists
  async checkUserExists(email) {
    const user = await this.db.query('SELECT id FROM users WHERE email = ?', [email]);
    return user !== null;
  }

  // Validate user data
  validateUserData(data) {
    if (!data.email || !this.isValidEmail(data.email)) {
      return false;
    }
    if (!data.name || data.name.length < 2) {
      return false;
    }
    return true;
  }
}

// Configuration validator
function isProductionConfig(config) {
  return config.env === 'production' && 
         config.ssl === true && 
         config.debug === false;
}

// Permission checker
function canUserEdit(user, resource) {
  if (user.isAdmin) return true;
  if (resource.ownerId === user.id) return true;
  return false;
}

// Export for use in other modules
module.exports = {
  UserService,
  isProductionConfig,
  canUserEdit
};