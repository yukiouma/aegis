import { Drawer, Box, Typography, IconButton, Divider } from '@mui/material';
import { FormatIndentDecrease, FormatIndentIncrease } from '@mui/icons-material';
import type { SidebarProps } from './types';

export function Sidebar({
  title,
  open,
  onToggle,
  width = 240,
  collapsedWidth = 56,
}: SidebarProps) {
  const drawerWidth = open ? width : collapsedWidth;

  return (
    <Drawer
      variant="permanent"
      data-testid="sidebar"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
        '& .MuiDrawer-paper': {
          width: drawerWidth,
          boxSizing: 'border-box',
          transition: 'width 0.3s',
          overflowX: 'hidden',
        },
      }}
    >
      <Box sx={{ display: 'flex', alignItems: 'center', p: 1, minHeight: 56 }}>
        <IconButton onClick={onToggle} aria-label="toggle sidebar" edge="start">
          {open ? <FormatIndentDecrease /> : <FormatIndentIncrease />}
        </IconButton>
        {open && (
          <Typography variant="h6" sx={{ ml: 1 }} noWrap>
            {title}
          </Typography>
        )}
      </Box>
      <Divider />
    </Drawer>
  );
}