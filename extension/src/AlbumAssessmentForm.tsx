import { useState } from "react";
import { Async } from "react-async";
import { assessAlbum } from "./core";
import { AppContextValue } from "./types";
import { AlbumAssessment } from "./proto/lute_pb";

const loadAlbumAssessment = async ({
  profileId,
  fileName,
}: {
  profileId: string;
  fileName: string | undefined;
}) => {
  if (!profileId || !fileName) {
    return null;
  }
  return await assessAlbum(profileId, fileName);
};

const formatScore = (score: number) => {
  return `${(score * 100).toFixed(3)}%`;
};

const VStack = ({
  gap = "0.5rem",
  children,
}: {
  gap?: string;
  children: React.ReactNode;
}) => (
  <div
    style={{
      display: "flex",
      flexDirection: "column",
      gap,
    }}
  >
    {children}
  </div>
);

export const AlbumAssessmentForm = ({
  context,
}: {
  context: AppContextValue;
}) => {
  const [profileId, setProfileId] = useState("main");

  return (
    <VStack gap="1rem">
      <div>
        <div>Profile</div>
        <div>
          <select
            value={profileId}
            onChange={(event) => setProfileId(event.target.value)}
          >
            {context.profiles.map((profile) => (
              <option key={profile.getId()} value={profile.getId()}>
                {profile.getName()}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div>
        <Async
          promiseFn={loadAlbumAssessment as any}
          profileId={profileId}
          fileName={context.page?.fileName}
          watch={profileId}
        >
          <Async.Rejected>
            {(error) => <div>Error: {error.message}</div>}
          </Async.Rejected>
          <Async.Fulfilled>
            {(assessment: AlbumAssessment) => (
              <VStack>
                <div>
                  <b>Score:</b> {formatScore(assessment.getScore())}
                </div>
                <details>
                  <summary>Breakdown</summary>
                  <VStack>
                    {assessment
                      .getMetadataMap()
                      .toArray()
                      .map(([key, value]) => (
                        <div key={key}>
                          <div>
                            <b>{key}</b>
                          </div>
                          <div>{formatScore(Number(value))}</div>
                        </div>
                      ))}
                  </VStack>
                </details>
              </VStack>
            )}
          </Async.Fulfilled>
        </Async>
      </div>
    </VStack>
  );
};
