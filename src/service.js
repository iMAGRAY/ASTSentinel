// Original service implementation
class UserService {
  constructor(db) {
    this.db = db;
  }
  
  async getUser(id) {
    const user = await this.db.query('SELECT * FROM users WHERE id = ?', [id]);
    return user[0];
  }
  
  async createUser(data) {
    const result = await this.db.query(
      'INSERT INTO users (name, email) VALUES (?, ?)',
      [data.name, data.email]
    );
    return { id: result.insertId, ...data };
  }
  
  async updateUser(id, data) {
    await this.db.query(
      'UPDATE users SET name = ?, email = ? WHERE id = ?',
      [data.name, data.email, id]
    );
    return { id, ...data };
  }
}

module.exports = UserService;