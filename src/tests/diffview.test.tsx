import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { DiffView } from '../components/DiffView';

describe('DiffView', () => {
  it('renders added and removed source lines', () => {
    render(<DiffView diff={'--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-old\n+new'} />);
    expect(screen.getByText('old')).toBeInTheDocument();
    expect(screen.getByText('new')).toBeInTheDocument();
  });
});
