-- Initial Schema for Admin Server
-- This is a consolidated migration that includes all database objects and initial data

-- Enable foreign key support
PRAGMA foreign_keys = ON;

-- =============================================
-- 1. User Types
-- =============================================
CREATE TABLE IF NOT EXISTS user_type (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    description TEXT,
    is_active   BOOLEAN DEFAULT TRUE NOT NULL,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- =============================================
-- 2. Admin Users
-- =============================================
CREATE TABLE IF NOT EXISTS admin_user (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    user_type_id  INTEGER NOT NULL REFERENCES user_type (id) ON DELETE RESTRICT,
    is_active     BOOLEAN DEFAULT TRUE NOT NULL,
    last_login_at DATETIME,
    created_at    DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- =============================================
-- 3. Permissions
-- =============================================
CREATE TABLE IF NOT EXISTS permission (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    code        TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    description TEXT,
    category    TEXT DEFAULT 'system',
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- =============================================
-- 4. User Type - Permission Mapping
-- =============================================
CREATE TABLE IF NOT EXISTS user_type_permission (
    user_type_id  INTEGER NOT NULL REFERENCES user_type (id) ON DELETE CASCADE,
    permission_id INTEGER NOT NULL REFERENCES permission (id) ON DELETE CASCADE,
    created_at    DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    PRIMARY KEY (user_type_id, permission_id)
);

-- =============================================
-- 5. Refresh Tokens
-- =============================================
CREATE TABLE IF NOT EXISTS user_refresh_token (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id       INTEGER NOT NULL REFERENCES admin_user (id) ON DELETE CASCADE,
    refresh_token TEXT NOT NULL,
    expires_at    DATETIME NOT NULL,
    created_at    DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at    DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(user_id)
);

-- =============================================
-- 6. Password Reset Tokens
-- =============================================
CREATE TABLE IF NOT EXISTS password_reset_token (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER NOT NULL REFERENCES admin_user (id) ON DELETE CASCADE,
    token       TEXT NOT NULL,
    expires_at  DATETIME NOT NULL,
    is_used     BOOLEAN DEFAULT FALSE NOT NULL,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(token)
);

-- =============================================
-- 7. History/Audit Log
-- =============================================
CREATE TABLE IF NOT EXISTS history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id     INTEGER REFERENCES admin_user (id) ON DELETE SET NULL,
    action      TEXT NOT NULL,
    entity_id   INTEGER,
    details     TEXT,
    ip_address  TEXT,
    user_agent  TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- =============================================
-- Indexes
-- =============================================
-- Admin User Indexes
CREATE INDEX IF NOT EXISTS idx_admin_user_user_type_id ON admin_user (user_type_id);
CREATE INDEX IF NOT EXISTS idx_admin_user_username ON admin_user (username);

-- Refresh Token Indexes
CREATE INDEX IF NOT EXISTS idx_refresh_token_user_id ON user_refresh_token (user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_token_token ON user_refresh_token (refresh_token);

-- Password Reset Token Indexes
CREATE INDEX IF NOT EXISTS idx_password_reset_token ON password_reset_token (token);

-- History/Audit Log Indexes
CREATE INDEX IF NOT EXISTS idx_history_user_id ON history (user_id);
CREATE INDEX IF NOT EXISTS idx_history_created_at ON history (created_at);
CREATE INDEX IF NOT EXISTS idx_history_entity_id ON history (entity_id);

-- =============================================
-- Triggers for updated_at
-- =============================================
-- User Type Triggers
CREATE TRIGGER IF NOT EXISTS user_type_updated_at
    AFTER UPDATE ON user_type
    FOR EACH ROW
BEGIN
    UPDATE user_type SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Admin User Triggers
CREATE TRIGGER IF NOT EXISTS admin_user_updated_at
    AFTER UPDATE ON admin_user
    FOR EACH ROW
BEGIN
    UPDATE admin_user SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- Permission Triggers
CREATE TRIGGER IF NOT EXISTS permission_updated_at
    AFTER UPDATE ON permission
    FOR EACH ROW
BEGIN
    UPDATE permission SET updated_at = CURRENT_TIMESTAMP WHERE id = OLD.id;
END;

-- =============================================
-- Initial Data
-- =============================================
-- Insert default user types
INSERT INTO user_type (code, name, description, is_active)
VALUES 
    ('super_admin', 'Super Administrator', 'Has full access to all features', 1),
    ('admin', 'Administrator', 'Has most administrative privileges', 1),
    ('manager', 'Manager', 'Has limited administrative access', 1),
    ('user', 'Standard User', 'Basic user with minimal permissions', 1)
ON CONFLICT(code) DO NOTHING;

-- Insert default permissions
INSERT INTO permission (code, name, description, category)
VALUES 
    -- User Management
    ('user:read', 'View Users', 'View user accounts', 'user'),
    ('user:create', 'Create Users', 'Create new user accounts', 'user'),
    ('user:update', 'Edit Users', 'Edit existing user accounts', 'user'),
    ('user:delete', 'Delete Users', 'Delete user accounts', 'user'),
    
    -- Role Management
    ('role:read', 'View Roles', 'View user roles and permissions', 'role'),
    ('role:create', 'Create Roles', 'Create new user roles', 'role'),
    ('role:update', 'Edit Roles', 'Edit existing user roles', 'role'),
    ('role:delete', 'Delete Roles', 'Delete user roles', 'role'),
    ('role:assign', 'Assign Roles', 'Assign roles to users', 'role'),
    
    -- System Settings
    ('settings:read', 'View Settings', 'View system settings', 'system'),
    ('settings:update', 'Update Settings', 'Update system settings', 'system'),
    
    -- Audit Logs
    ('audit:read', 'View Audit Logs', 'View system audit logs', 'audit')
ON CONFLICT(code) DO NOTHING;

-- Assign all permissions to super admin
INSERT INTO user_type_permission (user_type_id, permission_id)
SELECT 
    ut.id, 
    p.id 
FROM 
    user_type ut, 
    permission p 
WHERE 
    ut.code = 'super_admin' 
    AND p.code IN (
        'user:read', 'user:create', 'user:update', 'user:delete',
        'role:read', 'role:create', 'role:update', 'role:delete', 'role:assign',
        'settings:read', 'settings:update',
        'audit:read'
    )
ON CONFLICT(user_type_id, permission_id) DO NOTHING;

-- Create default admin user (password: admin123 - should be changed after first login)
INSERT INTO admin_user (username, password_hash, user_type_id, is_active)
SELECT 
    'admin', 
    -- bcrypt hash for 'admin123'
    '$2b$12$LQv3c1yNPlrxEEe4eJXZ.O9BKWrZwvsMQd.mBm9H7H4/4t3VZQ7Qa',
    (SELECT id FROM user_type WHERE code = 'super_admin' LIMIT 1),
    1
WHERE NOT EXISTS (SELECT 1 FROM admin_user WHERE username = 'admin');

-- Initialize first audit log entry
INSERT INTO history (user_id, action, entity_id, details, ip_address, user_agent)
VALUES (
    (SELECT id FROM admin_user WHERE username = 'admin' LIMIT 1),
    'SYSTEM_INIT',
    NULL,
    'System initialized with default admin user and permissions',
    '127.0.0.1',
    'System/1.0'
);

-- =============================================
-- Database Version
-- =============================================
-- This table tracks the database schema version
CREATE TABLE IF NOT EXISTS schema_migrations (
    version BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Mark this migration as applied
INSERT OR IGNORE INTO schema_migrations (version, name) 
VALUES (20240629000000, 'initial_schema');

-- Print success message
SELECT 'Database schema initialized successfully' as message;
