BEGIN; -- Work in a transaction

-- Table for the raw imported data
-- Requires some key fields, but allows any other data as JSONB
CREATE TABLE ImportedAccountChanges (
  id BIGSERIAL PRIMARY KEY,
  -- Connect each account change to a specific account directly,
  -- which also connects it to a Bookkeeping by extension
  account_id BIGINT NOT NULL,
  day DATE NOT NULL,
  -- Exactly same numeric as AccountChanges
  amount NUMERIC(32,2) NOT NULL,

  -- Other data included to help identify it, won't be kept post migration
  other_data JSONB DEFAULT '{}'::jsonb,

  FOREIGN KEY (account_id) REFERENCES Accounts(id)
);

COMMIT; -- Apply the transaction
