#!/bin/bash
# Script to update the draw_quad_points calls in lib.rs

# Find and replace the draw_quad_points function calls
sed -i '866s/, &self.config_uniform.bind_group//' /home/pv/brg/code/wgpu_viewer/src/lib.rs
sed -i '879s/, &self.config_uniform.bind_group//' /home/pv/brg/code/wgpu_viewer/src/lib.rs
