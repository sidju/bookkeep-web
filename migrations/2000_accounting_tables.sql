BEGIN; -- Work in a transaction

-- Base object of a bookkeeping, all others connect up to it in a tree of links
CREATE TABLE Bookkeepings (
  id BIGSERIAL PRIMARY KEY,
  name VARCHAR(64) NOT NULL,
  owner_id BIGINT NOT NULL,

  UNIQUE(name, owner_id),

  FOREIGN KEY (owner_id) REFERENCES Users(id)
);

-- Give a user other than the owner access to a bookkeeping
CREATE TABLE UsersBookkeepingsAccess (
  id BIGSERIAL PRIMARY KEY,
  user_id BIGINT NOT NULL,
  bookkeeping_id BIGINT NOT NULL,

  FOREIGN KEY (user_id) REFERENCES Users(id),
  FOREIGN KEY (bookkeeping_id) REFERENCES Bookkeepings(id)
);

--- Used as an enum, but sqlx doesn't handle postgres enums well
CREATE TABLE AccountTypes (
  name VARCHAR(64) PRIMARY KEY
);
INSERT INTO AccountTypes(name) VALUES ('Expense'),('Asset'),('Debt'),('Income');

-- Accounts as in accounting
CREATE TABLE Accounts (
  id BIGSERIAL PRIMARY KEY,
  bookkeeping_id BIGINT NOT NULL,
  name VARCHAR(64) NOT NULL,
  type VARCHAR(64) NOT NULL,

  UNIQUE (bookkeeping_id, name),

  FOREIGN KEY (bookkeeping_id) REFERENCES Bookkeepings(id),
  FOREIGN KEY (type) REFERENCES AccountTypes(name)
);

-- Additional way to segment transactions, for when dates aren't enough
-- Every Transaction belongs to a single Grouping
CREATE TABLE Groupings (
  id BIGSERIAL PRIMARY KEY,
  name VARCHAR(64) NOT NULL,
  bookkeeping_id BIGINT NOT NULL,
  comments JSONB DEFAULT '{}'::jsonb,

  UNIQUE (bookkeeping_id, name),

  FOREIGN KEY (bookkeeping_id) REFERENCES Bookkeepings(id)
);

-- One financial event, that the account changes it caused is linked to
CREATE TABLE Transactions (
  id BIGSERIAL PRIMARY KEY,
  name VARCHAR(64) NOT NULL,
  day DATE NOT NULL,
  bookkeeping_id BIGINT NOT NULL,
  grouping_id BIGINT NOT NULL,
  comments JSONB DEFAULT '{}'::jsonb,

  FOREIGN KEY (bookkeeping_id) REFERENCES Bookkeepings(id),
  FOREIGN KEY (grouping_id) REFERENCES Groupings(id)
);

-- One balance change on one account
-- Adjusted to be able to import bank statements
CREATE TABLE AccountChanges (
  id BIGSERIAL PRIMARY KEY,
  transaction_id BIGINT NOT NULL,
  account_id BIGINT NOT NULL,
  message VARCHAR(256) DEFAULT '',
  day DATE NOT NULL,
  -- Up to 32 digits, two of which after the decimal point
  amount NUMERIC(32,2) NOT NULL,

  FOREIGN KEY (transaction_id) REFERENCES Transactions(id),
  FOREIGN KEY (account_id) REFERENCES Accounts(id)
);

COMMIT; -- Apply the transaction
