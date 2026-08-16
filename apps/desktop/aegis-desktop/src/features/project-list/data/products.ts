import { useQuery } from "@tanstack/react-query";

import { api, type ApiError, type ProductView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

/**
 * All products. Consumed by the drawer's product dropdown. Inherits
 * the global staleTime — products rarely change.
 */
export function useListProducts() {
  return useQuery<ProductView[], ApiError>({
    queryKey: queryKeys.product.all(),
    queryFn: () => api.listProducts(),
  });
}