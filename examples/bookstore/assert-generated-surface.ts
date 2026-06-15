import {
  findBookDetail,
  type findBookDetail_Output,
  listAvailableBooks,
  type listAvailableBooks_Output,
  listBooksNeedingRestock,
  type listBooksNeedingRestock_Output,
  listTopRatedBooks,
  type listTopRatedBooks_Output,
} from "./generated/sql/books";
import {
  findLatestOrderForCustomer,
  type findLatestOrderForCustomer_Output,
  listCustomerOrders,
  type listCustomerOrders_Output,
  listMonthlySales,
  type listMonthlySales_Output,
  listUnreviewedPurchases,
  type listUnreviewedPurchases_Output,
} from "./generated/sql/orders";

const availableBooksQuery = listAvailableBooks();
const availableBooksSql: string = availableBooksQuery.sql;
const availableBooksParams: readonly [] = availableBooksQuery.params;
const availableBooksOutput: listAvailableBooks_Output = [];

const bookDetailQuery = findBookDetail({});
const bookDetailSql: string = bookDetailQuery.sql;
const bookDetailParams: readonly [] = bookDetailQuery.params;
const bookDetailOutput: findBookDetail_Output = null;

const restockQuery = listBooksNeedingRestock();
const restockOutput: listBooksNeedingRestock_Output = [];

const topRatedQuery = listTopRatedBooks();
const topRatedOutput: listTopRatedBooks_Output = [];

const customerOrdersQuery = listCustomerOrders();
const customerOrdersOutput: listCustomerOrders_Output = [];

const latestOrderQuery = findLatestOrderForCustomer();
const latestOrderOutput: findLatestOrderForCustomer_Output = null;

const unreviewedPurchasesQuery = listUnreviewedPurchases();
const unreviewedPurchasesOutput: listUnreviewedPurchases_Output = [];

const monthlySalesQuery = listMonthlySales();
const monthlySalesOutput: listMonthlySales_Output = [];

void availableBooksSql;
void availableBooksParams;
void availableBooksOutput;
void bookDetailSql;
void bookDetailParams;
void bookDetailOutput;
void restockQuery;
void restockOutput;
void topRatedQuery;
void topRatedOutput;
void customerOrdersQuery;
void customerOrdersOutput;
void latestOrderQuery;
void latestOrderOutput;
void unreviewedPurchasesQuery;
void unreviewedPurchasesOutput;
void monthlySalesQuery;
void monthlySalesOutput;
