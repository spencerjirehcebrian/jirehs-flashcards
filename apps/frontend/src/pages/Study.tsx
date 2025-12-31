import { useParams } from 'react-router-dom';
import { StudySession } from '../components/Study/StudySession';

export function Study() {
  const { deckPath } = useParams<{ deckPath?: string }>();
  const decodedPath = deckPath ? decodeURIComponent(deckPath) : undefined;

  return (
    <div className="study-page">
      {decodedPath && <h1 className="study-title">{decodedPath}</h1>}
      <StudySession deckPath={decodedPath} />
    </div>
  );
}
