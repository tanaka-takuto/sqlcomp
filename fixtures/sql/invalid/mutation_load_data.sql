/* @sqlay
{
  type: mutation
  id: mutationLoadData
}
*/
LOAD DATA INFILE '/tmp/fixture_child.csv'
INTO TABLE fixture_child
FIELDS TERMINATED BY ',';
