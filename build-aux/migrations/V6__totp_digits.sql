-- Add totp_digits column with default value of 6
ALTER TABLE accounts ADD COLUMN totp_digits INTEGER NOT NULL DEFAULT 6;

-- Update NON-STEAM accounts to use 6 digits
UPDATE accounts SET totp_digits = 6 WHERE algorithm <> 'STEAM';

-- Update STEAM accounts to use 5 digits
UPDATE accounts SET totp_digits = 5 WHERE algorithm = 'STEAM';
