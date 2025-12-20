-- Migration: Add is_admin columns to users and invite_codes tables
-- Run this on existing PostgreSQL databases to add admin flag support

-- Add is_admin to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;

-- Add is_admin to invite_codes table
ALTER TABLE invite_codes ADD COLUMN IF NOT EXISTS is_admin BOOLEAN NOT NULL DEFAULT FALSE;
