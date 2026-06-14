import {
  findLatestUser,
  type findLatestUser_Output,
  listUsers,
  type listUsers_Output,
} from "./generated/users";

const listUsersQuery = listUsers();
const listUsersSql: string = listUsersQuery.sql;
const listUsersParams: readonly [] = listUsersQuery.params;
const listUsersOutput: listUsers_Output = [
  { id: 1, name: "Ada" },
  { id: 2, name: null },
];

const findLatestUserQuery = findLatestUser({});
const findLatestUserSql: string = findLatestUserQuery.sql;
const findLatestUserParams: readonly [] = findLatestUserQuery.params;
const findLatestUserOutput: findLatestUser_Output = { id: 1, name: "Ada" };
const findLatestUserEmptyOutput: findLatestUser_Output = null;

void listUsersSql;
void listUsersParams;
void listUsersOutput;
void findLatestUserSql;
void findLatestUserParams;
void findLatestUserOutput;
void findLatestUserEmptyOutput;
