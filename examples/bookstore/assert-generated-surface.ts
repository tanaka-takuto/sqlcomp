import {
  findBookDetail,
  type findBookDetail_Output,
  listAvailableBooks,
  type listAvailableBooks_Input,
  type listAvailableBooks_Output,
  listBooksNeedingRestock,
  type listBooksNeedingRestock_Output,
  listTopRatedBooks,
  type listTopRatedBooks_Input,
  type listTopRatedBooks_Output,
} from "./generated/sql/books";
import {
  createOrder,
  type createOrderItems_Input,
  createOrderItems,
  deleteUnapprovedReview,
  findOrderById,
  type findOrderById_Output,
  findOrderByNumber,
  type findOrderByNumber_Output,
  markOrderPaid,
  replaceCategory,
  upsertOrderStatus,
} from "./generated/sql/mutations";
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

const availableBooksQuery = listAvailableBooks({});
const availableBooksSql: string = availableBooksQuery.sql;
const availableBooksParams: readonly unknown[] = availableBooksQuery.params;
const availableBooksOutput: listAvailableBooks_Output = [];
const availableBooksByFormatQuery = listAvailableBooks({
  discoveryFilter: { $fragment: "byBookFormat", format: "paperback" },
});
const availableBooksStaffPicksInput: listAvailableBooks_Input = {
  discoveryFilter: { $fragment: "staffPicksOnly" },
};
const availableBooksByIdsInput: listAvailableBooks_Input = {
  discoveryFilter: {
    $fragment: "byBookIds",
    ids: [{ id: "100" }, { id: "102" }],
  },
};
const availableBooksByIdsQuery = listAvailableBooks(availableBooksByIdsInput);

const bookDetailQuery = findBookDetail({ isbn: "9780441478125" });
const bookDetailSql: string = bookDetailQuery.sql;
const bookDetailParams: readonly [string] = bookDetailQuery.params;
const bookDetailOutput: findBookDetail_Output = null;

const restockQuery = listBooksNeedingRestock();
const restockOutput: listBooksNeedingRestock_Output = [];

const topRatedQuery = listTopRatedBooks({
  discoveryFilter: { $fragment: "staffPicksOnly" },
});
const topRatedOutput: listTopRatedBooks_Output = [];
const topRatedByFormatInput: listTopRatedBooks_Input = {
  discoveryFilter: { $fragment: "byBookFormat", format: "ebook" },
};
const topRatedByIdsInput: listTopRatedBooks_Input = {
  discoveryFilter: {
    $fragment: "byBookIds",
    ids: [{ id: "100" }],
  },
};

const customerOrdersQuery = listCustomerOrders();
const customerOrdersOutput: listCustomerOrders_Output = [];

const latestOrderQuery = findLatestOrderForCustomer();
const latestOrderOutput: findLatestOrderForCustomer_Output = null;

const unreviewedPurchasesQuery = listUnreviewedPurchases();
const unreviewedPurchasesOutput: listUnreviewedPurchases_Output = [];

const monthlySalesQuery = listMonthlySales();
const monthlySalesOutput: listMonthlySales_Output = [];

const createOrderMutation = createOrder({
  customerId: "1000",
  orderNumber: "BK-2000",
  status: "draft",
  currency: "USD",
  placedAt: "2026-04-20 12:00:00.000000",
  paidAt: null,
  shippedAt: null,
  shippingMethod: "priority",
  giftMessage: null,
});
const createOrderSql: string = createOrderMutation.sql;
const createOrderParams: readonly [
  string,
  string,
  string,
  string,
  string,
  string | null,
  string | null,
  string | null,
  string | null,
] = createOrderMutation.params;

const createdOrderQuery = findOrderById({ orderId: "5004" });
const createdOrderOutput: findOrderById_Output = null;
const upsertedOrderQuery = findOrderByNumber({ orderNumber: "BK-2001" });
const upsertedOrderOutput: findOrderByNumber_Output = null;

const markOrderPaidMutation = markOrderPaid({
  paidAt: "2026-04-20 12:01:00.000000",
  orderNumber: "BK-2000",
});
const deleteReviewMutation = deleteUnapprovedReview({ reviewId: "7003" });
const orderItemsInput: createOrderItems_Input = {
  items: [
    {
      orderId: "5004",
      bookId: "100",
      quantity: 1,
      unitPrice: "16.99",
      discountAmount: null,
    },
    {
      orderId: "5004",
      bookId: "102",
      quantity: 1,
      unitPrice: "18.00",
      discountAmount: "2.00",
    },
  ],
};
const orderItemsMutation = createOrderItems(orderItemsInput);
const orderItemsParams: readonly unknown[] = orderItemsMutation.params;
const upsertOrderStatusMutation = upsertOrderStatus({
  customerId: "1000",
  orderNumber: "BK-2001",
  initialStatus: "draft",
  currency: "USD",
  placedAt: "2026-04-20 12:00:00.000000",
  nextStatus: "paid",
  paidAt: "2026-04-20 12:01:00.000000",
});
const replaceCategoryMutation = replaceCategory({
  categoryId: "13",
  slug: "staff-picks",
  displayName: "Staff Picks",
});

void availableBooksSql;
void availableBooksParams;
void availableBooksOutput;
void availableBooksByFormatQuery;
void availableBooksStaffPicksInput;
void availableBooksByIdsQuery;
void bookDetailSql;
void bookDetailParams;
void bookDetailOutput;
void restockQuery;
void restockOutput;
void topRatedQuery;
void topRatedOutput;
void topRatedByFormatInput;
void topRatedByIdsInput;
void customerOrdersQuery;
void customerOrdersOutput;
void latestOrderQuery;
void latestOrderOutput;
void unreviewedPurchasesQuery;
void unreviewedPurchasesOutput;
void monthlySalesQuery;
void monthlySalesOutput;
void createOrderSql;
void createOrderParams;
void createdOrderQuery;
void createdOrderOutput;
void upsertedOrderQuery;
void upsertedOrderOutput;
void markOrderPaidMutation;
void deleteReviewMutation;
void orderItemsMutation;
void orderItemsParams;
void upsertOrderStatusMutation;
void replaceCategoryMutation;
