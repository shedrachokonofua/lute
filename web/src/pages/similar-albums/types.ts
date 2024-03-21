import { AlbumSearchFiltersForm } from "../../forms";

export interface SimilarAlbumsForm {
  fileName?: string;
  embeddingKey?: string;
  filters?: AlbumSearchFiltersForm;
  limit?: number;
}
