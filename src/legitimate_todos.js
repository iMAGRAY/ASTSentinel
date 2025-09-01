// Legitimate TODOs should be allowed now
class UserService {
  constructor(database) {
    this.db = database;
  }

  // TODO: (JIRA-1234) Add caching layer after Redis deployment
  async getUser(id) {
    if (!id) throw new Error("User ID required");
    return await this.db.query('SELECT * FROM users WHERE id = ?', [id]);
  }

  // TODO: #567 Optimize query performance 
  async getUsersByRole(role) {
    return await this.db.query('SELECT * FROM users WHERE role = ?', [role]);
  }

  // TODO: (@john) Review security implications
  validatePermissions(user, action) {
    return user.roles && user.roles.includes(action);
  }
}