import { useQuery } from "@tanstack/react-query";

import { api, type ApiError, type ProductView } from "../api";
import { queryKeys } from "./queryKeys";

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