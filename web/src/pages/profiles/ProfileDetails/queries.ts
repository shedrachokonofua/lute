import {
  getPendingSpotifyImports,
  getProfile,
  getProfileSummary,
} from "../../../client";

export const profileDetailsQuery = (id: string) => ({
  queryKey: ["profile", id],
  queryFn: async () => {
    const [profile, profileSummary] = await Promise.all([
      getProfile(id),
      getProfileSummary(id),
    ]);
    if (!profile || !profileSummary) {
      throw new Error("Profile not found");
    }
    return {
      profile,
      profileSummary,
    };
  },
});

export const pendingSpotifyImportsQuery = (profileId: string) => ({
  queryKey: ["profile", profileId, "pendingSpotifyImports"],
  queryFn: async () => getPendingSpotifyImports(profileId),
});
