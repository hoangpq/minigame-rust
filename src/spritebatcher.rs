use sdl2::render::RendererContext;
use sdl2::render::Canvas;
use sdl2::video::Window;
use graphicsdevice::GraphicsDevice;
use spritebatchitem::SpriteBatchItem;
use spritebatch::SpriteSortMode;
use spritebatch::SpriteBatch;
use renderstate::RenderState;
use texture::Texture;
use std::i32;
use std::vec;
use vertexpositioncolortexture::VertexPositionColorTexture;

pub struct SpriteBatcher<'a> {
    initial_batch_size: i32,
    max_batch_size: i32,
    initial_vertex_array_size: i32,
    renderer: &'a Canvas<Window>,
    graphics_device: GraphicsDevice,
    batch_item_list: Vec<SpriteBatchItem<'a>>, /// The list of batch items to process.
    batch_item_count: i32, /// Index pointer to the next available SpriteBatchItem in _batchItemList.
    index: Vec<i32>, /// Vertex index array. The values in this array never change.
    vertex_array: Vec<VertexPositionColorTexture>,
}

impl<'a> SpriteBatcher<'a> {
    pub fn new(renderer: &'a Canvas<Window>, graphics_device: GraphicsDevice) -> SpriteBatcher<'a> {
        let mut bil = Vec::new();
        for i in 0..256 {
            bil.push(SpriteBatchItem::new());
        }
        
        let mut sb = SpriteBatcher {
            initial_batch_size: 256,
            max_batch_size: i32::MAX / 6, // 6 = 4 vertices unique and 2 shared, per quad
            initial_vertex_array_size: 256, 
            renderer: renderer,
            graphics_device: graphics_device,
            batch_item_list: bil,
            batch_item_count: 0,
            index: Vec::new(),
            vertex_array: Vec::new(),
        };

        sb.ensure_array_capacity(256);

        sb
    }

    pub fn create_batch_item() -> SpriteBatchItem<'a> {
        SpriteBatchItem::new()
    }

    pub fn ensure_array_capacity(&mut self, num_batch_items: i32) {
        let neededCapacity = 6 * num_batch_items;
        if neededCapacity <= self.index.len() as i32 {
            // Short circuit out of here because we have enough capacity.
            return;
        }

        let mut newIndex: Vec<i32> = Vec::with_capacity(neededCapacity as usize);
        let start = 0;

        for i in 0..self.index.len() as usize {
            newIndex.push(self.index[i]);
        }

        let start = self.index.len() / 6;

        for i in start..num_batch_items as usize {
            /*
            *  TL    TR
            *   0----1 0,1,2,3 = index offsets for vertex indices
            *   |   /| TL,TR,BL,BR are vertex references in SpriteBatchItem.
            *   |  / |
            *   | /  |
            *   |/   |
            *   2----3
            *  BL    BR
            */
            // Triangle 1
            newIndex.insert((i * 6 + 0) as usize, (i * 4) as i32);
            newIndex.insert((i * 6 + 1) as usize, (i * 4 + 1) as i32);
            newIndex.insert((i * 6 + 2) as usize, (i * 4 + 2) as i32);
            // Triangle 2
            newIndex.insert((i * 6 + 3) as usize, (i * 4 + 1) as i32);
            newIndex.insert((i * 6 + 4) as usize, (i * 4 + 3) as i32);
            newIndex.insert((i * 6 + 5) as usize, (i * 4 + 2) as i32);
        }
        self.index = newIndex;

        self.vertex_array.resize(neededCapacity as usize, VertexPositionColorTexture::new());
    }

    pub fn draw_batch(&'a mut self, sort_mode: SpriteSortMode/*, Effect effect*/, render_state: &'a mut RenderState<'a>) {
        // nothing to do
        if self.batch_item_count == 0 {
            return;
        }

        // sort the batch items
        match sort_mode {
            SpriteSortMode::SpriteSortModeTexture => self.batch_item_list.sort_by(|a, b| a.cmp(b)),
            SpriteSortMode::SpriteSortModeFrontToBack => self.batch_item_list.sort_by(|a, b| a.cmp(b)),
            SpriteSortMode::SpriteSortModeBackToFront => self.batch_item_list.sort_by(|a, b| a.cmp(b)),
            _ => self.batch_item_list.sort_by(|a, b| a.cmp(b)),
        }

        // Determine how many iterations through the drawing code we need to make
        let mut batch_index: i32 = 0;
        let mut batch_count: i32 = self.batch_item_count;

        // Iterate through the batches, doing short.MaxValue sets of vertices only.
        while batch_count > 0 {
            // setup the vertexArray array
            let mut startIndex: i32 = 0;
            let mut index: i32 = 0;
            let mut tex: Option<&'a Texture<'a>> = None;

            let mut numBatchesToProcess: i32 = batch_count;
            if numBatchesToProcess > self.max_batch_size {
                numBatchesToProcess = self.max_batch_size;
            }
            
            {
                self.ensure_array_capacity(numBatchesToProcess);
            }

            // Draw the batches
            for i in 0..numBatchesToProcess {
                // if the texture changed, we need to flush and bind the new texture
                let shouldFlush: bool = &**self.batch_item_list[batch_index as usize].texture.as_ref().unwrap() as *const _ != &**tex.as_ref().unwrap() as *const _;
                if shouldFlush {
                    self.flush_vertex_array(startIndex, index /*, effect*/, tex, render_state);

                    tex = self.batch_item_list[batch_index as usize].texture;
                    startIndex = 0;
                    index = 0;
                }

                let mut item = &mut self.batch_item_list[batch_index as usize];
                // store the SpriteBatchItem data in our vertexArray
                index = index + 1;
                self.vertex_array[index as usize] = item.vertexTL;
                index = index + 1;
                self.vertex_array[index as usize] = item.vertexTR;
                index = index + 1;
                self.vertex_array[index as usize] = item.vertexBL;
                index = index + 1;
                self.vertex_array[index as usize] = item.vertexTR;
                index = index + 1;
                self.vertex_array[index as usize] = item.vertexBR;
                index = index + 1;
                self.vertex_array[index as usize] = item.vertexBL;

                // Release the texture.
                item.set_texture(None);
            }
            // flush the remaining vertexArray data
            self.flush_vertex_array(startIndex, index /*, effect*/, tex, render_state);
            // Update our batch count to continue the process of culling down
            // large batches
            batch_count -= numBatchesToProcess;
        }
        // return items to the pool.
        self.batch_item_count = 0;
    }

    pub fn flush_vertex_array(&mut self, start: i32, end: i32 /*, Effect effect*/, texture: Option<&'a Texture<'a>>, render_state: &mut RenderState<'a>) {
        if start == end {
            return;
        }

        let vertexCount: i32 = end - start;
        render_state.set_texture(texture);
        
        self.graphics_device.draw(&self.vertex_array, vertexCount, render_state);
    }
  

}