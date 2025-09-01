// Production test - legitimate code should pass
class UserService {
  constructor(database) {
    this.db = database;
  }
  
  async getUser(id) {
    if (!id) return null;
    
    try {
      const user = await this.db.query('SELECT * FROM users WHERE id = ?', [id]);
      return user;
    } catch (error) {
      console.error('Database error:', error);
      return null;
    }
  }
}

module.exports = UserService;