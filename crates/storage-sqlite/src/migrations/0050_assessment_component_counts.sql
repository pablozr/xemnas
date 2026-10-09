-- What became of the map components the extractor named in each run, as counts
-- only (no names, no text): the size of the map it was shown, the names it
-- proposed, the ones kept, the ones the map does not list and the ones whose
-- quote the candidate's own text does not back. NULL for runs before this.
ALTER TABLE assessments ADD COLUMN components_listed INTEGER;
ALTER TABLE assessments ADD COLUMN components_proposed INTEGER;
ALTER TABLE assessments ADD COLUMN components_kept INTEGER;
ALTER TABLE assessments ADD COLUMN components_unknown INTEGER;
ALTER TABLE assessments ADD COLUMN components_unquoted INTEGER;
