# Add support for account change imports:

- Add a link to account change imports under each grouping.
  In the page linked to:
  - Add a form to import a CSV of account changes
    (Copy as is from current groupings index.html ?)
  - Show a form to create a transaction from a selection of imported changes:
    - Transaction Name (optional, defaults to first change's description)
    - Transaction Date (optional, defaults to first change's date)
    - Account upon which to create the balancing account change (required if
      the transaction isn't already balanced)
    - A selection of which imported account changes to include in the
      transaction, listed with the following per line:
      - Description
      - The resulting balance on the account
      - The resulting balance that the import says should be on the account
      - A checkbox, which adds the ID of this line's entry to the form
      - A delete button, kept safely separate, to delete this line's entry.
        (since duplicates are likely to occur it should be easy)
    - A create button, which creates the defined transaction and it's account
      changes as defined by the form + import data.

# Reconsider session ID generation:

`nanoid` is fine, but potential improvements are:
- `ULID`, less random with inherent time sortability (maybe better for DB perf)
- `CUID2`, better randomness should give higher security and less collisions

# Fix cacheing headers:

Currently at least firefox caches, which causes basically everything to be
outdated most the time...
