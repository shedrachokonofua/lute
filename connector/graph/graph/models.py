from dataclasses import dataclass

from pydantic import BaseModel, PositiveInt


class AlbumRelationWeights(BaseModel):
    album_artist: PositiveInt = 4
    credited: PositiveInt = 2
    descriptor: PositiveInt = 1
    language: PositiveInt = 1


@dataclass
class EmbeddingDocument:
    file_name: str
    embedding: list[float]
    embedding_key: str

    def is_zero_magnitude(self):
        return all(v == 0.0 for v in self.embedding)
